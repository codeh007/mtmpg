use std::env;
use std::error::Error;
use std::fs::OpenOptions;
use std::io::{Error as IoError, ErrorKind, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use jaws::Token;
use p256::ecdsa::{Signature, SigningKey};
use pggomtm::database_auth::{AuthMethod, DatabaseProfile, DatabaseTokenClaims};

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = env::args_os().skip(1);
    let valid_path = PathBuf::from(arguments.next().ok_or_else(|| {
        IoError::new(
            ErrorKind::InvalidInput,
            "expected valid and tampered token output paths",
        )
    })?);
    let tampered_path = PathBuf::from(arguments.next().ok_or_else(|| {
        IoError::new(
            ErrorKind::InvalidInput,
            "expected a tampered token output path",
        )
    })?);
    if arguments.next().is_some() {
        return Err(IoError::new(ErrorKind::InvalidInput, "unexpected argument").into());
    }

    let now = i64::try_from(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())?;
    let claims = DatabaseTokenClaims {
        issuer: "https://candidate.example.test/oauth/database".into(),
        audience: "https://candidate.example.test/resources/database/gomtm-test".into(),
        subject: "usr_pgx_gate".into(),
        issued_at: now.saturating_sub(1),
        expires_at: now.saturating_add(119),
        token_id: "jti_pgx_gate".into(),
        scope: "database".into(),
        delegation_id: "dlg_pgx_gate".into(),
        auth_method: AuthMethod::OAuth,
        authority_version: 1,
        db_profile: DatabaseProfile::Ordinary,
        db_role: DatabaseProfile::Ordinary.database_role().into(),
        client_id: Some("cli_pgx_gate".into()),
        credential_id: None,
    };
    let key = SigningKey::from_slice(&[7_u8; 32])?;
    let mut token = Token::compact((), claims);
    *token.header_mut().key_id() = Some("candidate-es256-pgx-gate".into());
    *token.header_mut().r#type() = Some("JWT".into());
    let valid = token.sign::<_, Signature>(&key)?.rendered()?;

    let mut tampered = valid.as_bytes().to_vec();
    let signature_start = valid
        .rfind('.')
        .map(|position| position + 1)
        .ok_or_else(|| {
            IoError::new(
                ErrorKind::InvalidData,
                "signed token has no signature segment",
            )
        })?;
    let signature_byte = tampered.get_mut(signature_start).ok_or_else(|| {
        IoError::new(
            ErrorKind::InvalidData,
            "signed token has an empty signature",
        )
    })?;
    *signature_byte = if *signature_byte == b'A' { b'B' } else { b'A' };

    write_ephemeral_fixture(&valid_path, valid.as_bytes())?;
    write_ephemeral_fixture(&tampered_path, &tampered)?;
    println!("已生成仅供本次PG18.4 OAuth smoke使用的临时合成fixture");
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
