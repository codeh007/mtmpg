use std::env;
use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::{Error as IoError, ErrorKind, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use jaws::Token;
use p256::ecdsa::{Signature, SigningKey};
use pggomtm::database_auth::{
    AuthMethod, AuthenticatedActor, AuthenticatedIdentity, DatabaseProfile, DatabaseTokenClaims,
    MAX_AUTHN_ID_BYTES, decode_system_user,
};
use serde::Serialize;
use serde_json::Value;

const ISSUER: &str = "https://candidate.example.test/oauth/database";
const AUDIENCE: &str = "https://candidate.example.test/resources/database/gomtm-test";
const KID: &str = "candidate-es256-pgx-gate";
const SCENARIO: &str = "oauth-ordinary";
const SUBJECT: &str = "usr_oauth_ordinary";
const CLIENT_ID: &str = "cli_oauth_ordinary";
const DELEGATION_ID: &str = "dlg_oauth_ordinary";

fn ordinary_claims(now: i64) -> DatabaseTokenClaims {
    DatabaseTokenClaims {
        issuer: ISSUER.into(),
        audience: AUDIENCE.into(),
        subject: SUBJECT.into(),
        issued_at: now.saturating_sub(1),
        expires_at: now.saturating_add(299),
        token_id: format!("jti_{SCENARIO}"),
        scope: "database".into(),
        delegation_id: DELEGATION_ID.into(),
        auth_method: AuthMethod::OAuth,
        authority_version: 1,
        db_profile: DatabaseProfile::Ordinary,
        db_role: DatabaseProfile::Ordinary.database_role().into(),
        client_id: Some(CLIENT_ID.into()),
        credential_id: None,
    }
}

fn ordinary_identity() -> AuthenticatedIdentity {
    AuthenticatedIdentity {
        user_id: SUBJECT.into(),
        actor: AuthenticatedActor::OAuthClient(CLIENT_ID.into()),
        delegation_id: DELEGATION_ID.into(),
        auth_method: AuthMethod::OAuth,
        authority_version: 1,
        profile: DatabaseProfile::Ordinary,
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = env::args_os().skip(1);
    let command = arguments
        .next()
        .ok_or_else(|| invalid_input("expected a fixture command"))?;
    let command = command
        .to_str()
        .ok_or_else(|| invalid_input("fixture command must be UTF-8"))?;

    match command {
        "generate" => {
            let output_dir = PathBuf::from(
                arguments
                    .next()
                    .ok_or_else(|| invalid_input("expected an output directory"))?,
            );
            reject_extra_arguments(arguments)?;
            generate_fixtures(&output_dir)?;
        }
        "verify-system-user" => {
            let slug = arguments
                .next()
                .ok_or_else(|| invalid_input("expected an identity scenario"))?;
            let slug = slug
                .to_str()
                .ok_or_else(|| invalid_input("identity scenario must be UTF-8"))?;
            let path = PathBuf::from(
                arguments
                    .next()
                    .ok_or_else(|| invalid_input("expected a system_user fixture"))?,
            );
            reject_extra_arguments(arguments)?;
            verify_system_user(slug, &path)?;
        }
        _ => return Err(invalid_input("unknown fixture command").into()),
    }

    Ok(())
}

fn generate_fixtures(output_dir: &Path) -> Result<(), Box<dyn Error>> {
    if !output_dir.is_dir() {
        return Err(invalid_input("fixture output directory does not exist").into());
    }

    let now = i64::try_from(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())?;
    let key = SigningKey::from_slice(&[7_u8; 32])?;
    let ordinary_oauth_token = sign_claims(ordinary_claims(now), &key)?;
    write_ephemeral_fixture(
        &output_dir.join(format!("{SCENARIO}.jwt")),
        ordinary_oauth_token.as_bytes(),
    )?;

    for (scenario, profile, role) in [
        ("oauth-v1-profile", "business-admin", "gomtm_candidate_business_admin"),
        ("oauth-project-role", "ordinary", "gomtm_ordinary"),
        ("oauth-stage-role", "ordinary", "gomtm_candidate_ordinary"),
    ] {
        let token = sign_named_claims(now, profile, role, &key)?;
        write_ephemeral_fixture(
            &output_dir.join(format!("{scenario}.jwt")),
            token.as_bytes(),
        )?;
    }

    let mut tampered = ordinary_oauth_token.into_bytes();
    let signature_start = tampered
        .iter()
        .rposition(|byte| *byte == b'.')
        .map(|position| position + 1)
        .ok_or_else(|| IoError::new(ErrorKind::InvalidData, "signed token has no signature"))?;
    let signature_byte = tampered.get_mut(signature_start).ok_or_else(|| {
        IoError::new(
            ErrorKind::InvalidData,
            "signed token has an empty signature",
        )
    })?;
    *signature_byte = if *signature_byte == b'A' { b'B' } else { b'A' };
    write_ephemeral_fixture(&output_dir.join("tampered.jwt"), &tampered)?;

    println!("已生成仅供本次PG18 OAuth矩阵使用的临时合成fixture");
    Ok(())
}

fn sign_named_claims(
    now: i64,
    profile: &str,
    role: &str,
    key: &SigningKey,
) -> Result<String, Box<dyn Error>> {
    let mut claims = serde_json::to_value(ordinary_claims(now))?;
    let object = claims
        .as_object_mut()
        .ok_or_else(|| invalid_input("database claims must be a JSON object"))?;
    object.insert("db_profile".into(), Value::String(profile.into()));
    object.insert("db_role".into(), Value::String(role.into()));
    sign_claims(claims, key)
}

fn sign_claims(claims: impl Serialize, key: &SigningKey) -> Result<String, Box<dyn Error>> {
    let mut token = Token::compact((), claims);
    *token.header_mut().key_id() = Some(KID.into());
    *token.header_mut().r#type() = Some("JWT".into());
    Ok(token.sign::<_, Signature>(key)?.rendered()?)
}

fn verify_system_user(slug: &str, path: &Path) -> Result<(), Box<dyn Error>> {
    if slug != SCENARIO {
        return Err(invalid_input("unknown identity scenario").into());
    }
    let system_user = fs::read_to_string(path)?;
    if system_user.is_empty() || system_user.len() > MAX_AUTHN_ID_BYTES + "oauth:".len() {
        return Err(IoError::new(ErrorKind::InvalidData, "invalid system_user length").into());
    }

    let decoded = decode_system_user(&system_user)?;
    let expected = ordinary_identity();
    if decoded != expected {
        return Err(IoError::new(ErrorKind::InvalidData, "decoded identity mismatch").into());
    }
    let canonical = format!("oauth:{}", expected.encode_authn_id()?);
    if system_user != canonical {
        return Err(IoError::new(ErrorKind::InvalidData, "non-canonical system_user").into());
    }

    println!("PG18 system_user identity round-trip passed for {slug}");
    Ok(())
}

fn write_ephemeral_fixture(path: &Path, value: &[u8]) -> Result<(), Box<dyn Error>> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(value)?;
    file.sync_all()?;
    Ok(())
}

fn reject_extra_arguments(
    mut arguments: impl Iterator<Item = std::ffi::OsString>,
) -> Result<(), IoError> {
    if arguments.next().is_some() {
        return Err(invalid_input("unexpected argument"));
    }
    Ok(())
}

fn invalid_input(message: &'static str) -> IoError {
    IoError::new(ErrorKind::InvalidInput, message)
}
