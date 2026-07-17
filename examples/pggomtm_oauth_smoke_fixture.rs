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

const ISSUER: &str = "https://candidate.example.test/oauth/database";
const AUDIENCE: &str = "https://candidate.example.test/resources/database/gomtm-test";
const KID: &str = "candidate-es256-pgx-gate";

#[derive(Clone, Copy)]
struct IdentityScenario {
    slug: &'static str,
    subject: &'static str,
    actor_id: &'static str,
    delegation_id: &'static str,
    auth_method: AuthMethod,
    authority_version: u64,
    profile: DatabaseProfile,
}

const IDENTITY_SCENARIOS: [IdentityScenario; 6] = [
    IdentityScenario {
        slug: "oauth-ordinary",
        subject: "usr_oauth_ordinary",
        actor_id: "cli_oauth_ordinary",
        delegation_id: "dlg_oauth_ordinary",
        auth_method: AuthMethod::OAuth,
        authority_version: 1,
        profile: DatabaseProfile::Ordinary,
    },
    IdentityScenario {
        slug: "oauth-business-admin",
        subject: "usr_oauth_business_admin",
        actor_id: "cli_oauth_business_admin",
        delegation_id: "dlg_oauth_business_admin",
        auth_method: AuthMethod::OAuth,
        authority_version: 2,
        profile: DatabaseProfile::BusinessAdmin,
    },
    IdentityScenario {
        slug: "oauth-database-developer",
        subject: "usr_oauth_database_developer",
        actor_id: "cli_oauth_database_developer",
        delegation_id: "dlg_oauth_database_developer",
        auth_method: AuthMethod::OAuth,
        authority_version: 3,
        profile: DatabaseProfile::DatabaseDeveloper,
    },
    IdentityScenario {
        slug: "api-key-ordinary",
        subject: "usr_api_key_ordinary",
        actor_id: "crd_api_key_ordinary",
        delegation_id: "dlg_api_key_ordinary",
        auth_method: AuthMethod::ApiKey,
        authority_version: 4,
        profile: DatabaseProfile::Ordinary,
    },
    IdentityScenario {
        slug: "api-key-business-admin",
        subject: "usr_api_key_business_admin",
        actor_id: "crd_api_key_business_admin",
        delegation_id: "dlg_api_key_business_admin",
        auth_method: AuthMethod::ApiKey,
        authority_version: 5,
        profile: DatabaseProfile::BusinessAdmin,
    },
    IdentityScenario {
        slug: "api-key-database-developer",
        subject: "usr_api_key_database_developer",
        actor_id: "crd_api_key_database_developer",
        delegation_id: "dlg_api_key_database_developer",
        auth_method: AuthMethod::ApiKey,
        authority_version: 6,
        profile: DatabaseProfile::DatabaseDeveloper,
    },
];

impl IdentityScenario {
    fn claims(self, now: i64) -> DatabaseTokenClaims {
        let (client_id, credential_id) = match self.auth_method {
            AuthMethod::OAuth => (Some(self.actor_id.into()), None),
            AuthMethod::ApiKey => (None, Some(self.actor_id.into())),
        };
        DatabaseTokenClaims {
            issuer: ISSUER.into(),
            audience: AUDIENCE.into(),
            subject: self.subject.into(),
            issued_at: now.saturating_sub(1),
            expires_at: now.saturating_add(299),
            token_id: format!("jti_{}", self.slug),
            scope: "database".into(),
            delegation_id: self.delegation_id.into(),
            auth_method: self.auth_method,
            authority_version: self.authority_version,
            db_profile: self.profile,
            db_role: self.profile.database_role().into(),
            client_id,
            credential_id,
        }
    }

    fn identity(self) -> AuthenticatedIdentity {
        let actor = match self.auth_method {
            AuthMethod::OAuth => AuthenticatedActor::OAuthClient(self.actor_id.into()),
            AuthMethod::ApiKey => AuthenticatedActor::ApiKeyCredential(self.actor_id.into()),
        };
        AuthenticatedIdentity {
            user_id: self.subject.into(),
            actor,
            delegation_id: self.delegation_id.into(),
            auth_method: self.auth_method,
            authority_version: self.authority_version,
            profile: self.profile,
        }
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
        "verify-codec-rejections" => {
            reject_extra_arguments(arguments)?;
            verify_codec_rejections()?;
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
    let mut ordinary_oauth_token = None;

    for scenario in IDENTITY_SCENARIOS {
        let token = sign_claims(scenario.claims(now), &key)?;
        write_ephemeral_fixture(
            &output_dir.join(format!("{}.jwt", scenario.slug)),
            token.as_bytes(),
        )?;
        if scenario.slug == "oauth-ordinary" {
            ordinary_oauth_token = Some(token);
        }
    }

    let mut overlong = IDENTITY_SCENARIOS[0].claims(now);
    overlong.subject = "x".repeat(65);
    write_ephemeral_fixture(
        &output_dir.join("invalid-overlong-identity.jwt"),
        sign_claims(overlong, &key)?.as_bytes(),
    )?;

    let mut illegal = IDENTITY_SCENARIOS[0].claims(now);
    illegal.delegation_id = "dlg:illegal".into();
    write_ephemeral_fixture(
        &output_dir.join("invalid-illegal-identity.jwt"),
        sign_claims(illegal, &key)?.as_bytes(),
    )?;

    let mut tampered = ordinary_oauth_token
        .ok_or_else(|| IoError::new(ErrorKind::InvalidData, "missing ordinary OAuth token"))?
        .into_bytes();
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

    println!("已生成仅供本次PG18.4 OAuth矩阵使用的临时合成fixture");
    Ok(())
}

fn sign_claims(claims: DatabaseTokenClaims, key: &SigningKey) -> Result<String, Box<dyn Error>> {
    let mut token = Token::compact((), claims);
    *token.header_mut().key_id() = Some(KID.into());
    *token.header_mut().r#type() = Some("JWT".into());
    Ok(token.sign::<_, Signature>(key)?.rendered()?)
}

fn verify_system_user(slug: &str, path: &Path) -> Result<(), Box<dyn Error>> {
    let scenario = IDENTITY_SCENARIOS
        .into_iter()
        .find(|scenario| scenario.slug == slug)
        .ok_or_else(|| invalid_input("unknown identity scenario"))?;
    let system_user = fs::read_to_string(path)?;
    if system_user.is_empty() || system_user.len() > MAX_AUTHN_ID_BYTES + "oauth:".len() {
        return Err(IoError::new(ErrorKind::InvalidData, "invalid system_user length").into());
    }

    let decoded = decode_system_user(&system_user)?;
    let expected = scenario.identity();
    if decoded != expected {
        return Err(IoError::new(ErrorKind::InvalidData, "decoded identity mismatch").into());
    }
    let canonical = format!("oauth:{}", expected.encode_authn_id()?);
    if system_user != canonical {
        return Err(IoError::new(ErrorKind::InvalidData, "non-canonical system_user").into());
    }

    println!("PG18.4 system_user identity round-trip passed for {slug}");
    Ok(())
}

fn verify_codec_rejections() -> Result<(), Box<dyn Error>> {
    let valid = IDENTITY_SCENARIOS[0].identity().encode_authn_id()?;
    let unknown_version = format!("oauth:{}", valid.replacen("pggomtm:v1", "pggomtm:v2", 1));
    let oversize = format!("oauth:{}", "x".repeat(MAX_AUTHN_ID_BYTES + 1));
    let illegal =
        "oauth:pggomtm:v1;u=bad:delimiter;actor=client:cli_ok;d=dlg_ok;m=oauth;a=1;p=ordinary";

    for rejected in [unknown_version.as_str(), oversize.as_str(), illegal] {
        if decode_system_user(rejected).is_ok() {
            return Err(IoError::new(
                ErrorKind::InvalidData,
                "invalid system_user decoded successfully",
            )
            .into());
        }
    }

    println!("PG18.4 system_user codec rejection matrix passed");
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
