use std::env;
use std::error::Error;
use std::fs::OpenOptions;
use std::io::{Error as IoError, ErrorKind, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use jaws::key::JsonWebKey;
use p256::SecretKey;
use p256::ecdsa::SigningKey;
use p256::elliptic_curve::pkcs8::EncodePrivateKey;
use serde_json::json;

const ISSUER: &str = "https://auth.example.test/database";
const AUDIENCE: &str = "https://postgres.example.test/database/main";
const KEY_ID: &str = "executor-es256-test";
const HMAC_SECRET: &[u8; 32] = &[0x42; 32];

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = env::args_os().skip(1);
    let command = arguments
        .next()
        .ok_or_else(|| invalid_input("expected fixture command"))?;
    let command = command
        .to_str()
        .ok_or_else(|| invalid_input("fixture command must be UTF-8"))?;
    if command != "generate" {
        return Err(invalid_input("unknown fixture command").into());
    }
    let output = PathBuf::from(
        arguments
            .next()
            .ok_or_else(|| invalid_input("expected output directory"))?,
    );
    if arguments.next().is_some() {
        return Err(invalid_input("unexpected fixture argument").into());
    }
    generate(&output)
}

fn generate(output: &Path) -> Result<(), Box<dyn Error>> {
    if !output.is_dir() {
        return Err(invalid_input("fixture output directory is unavailable").into());
    }

    let secret_key = SecretKey::from_slice(&[9_u8; 32])?;
    let signing_key = SigningKey::from_slice(&[9_u8; 32])?;
    let private_pem = secret_key.to_pkcs8_pem(Default::default())?;
    write_private(
        &output.join("signing-key.pem"),
        private_pem.as_str().as_bytes(),
    )?;
    write_private(&output.join("hmac.secret"), HMAC_SECRET)?;

    let mut jwk = serde_json::to_value(JsonWebKey::build(signing_key.verifying_key()))?;
    let jwk_object = jwk
        .as_object_mut()
        .ok_or_else(|| invalid_input("public JWK must be an object"))?;
    jwk_object.insert("alg".into(), json!("ES256"));
    jwk_object.insert("key_ops".into(), json!(["verify"]));
    jwk_object.insert("kid".into(), json!(KEY_ID));
    jwk_object.insert("use".into(), json!("sig"));
    write_public_json(&output.join("jwks.json"), &json!({"keys": [jwk]}))?;
    write_public_json(
        &output.join("validator.json"),
        &json!({
            "schema": "pggomtm-validator-config/v1",
            "issuer": ISSUER,
            "audience": AUDIENCE,
            "jwks_path": "/etc/pggomtm/jwks.json"
        }),
    )?;

    println!("generated ephemeral executor integration fixtures");
    Ok(())
}

fn write_private(path: &Path, value: &[u8]) -> Result<(), IoError> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o400)
        .open(path)?;
    file.write_all(value)?;
    file.sync_all()
}

fn write_public_json(path: &Path, value: &serde_json::Value) -> Result<(), Box<dyn Error>> {
    let encoded = serde_json::to_vec(value)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o444)
        .open(path)?;
    file.write_all(&encoded)?;
    file.sync_all()?;
    Ok(())
}

fn invalid_input(message: &'static str) -> IoError {
    IoError::new(ErrorKind::InvalidInput, message)
}
