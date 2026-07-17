use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use jaws::Token;
use jaws::key::JsonWebKey;
use p256::ecdsa::{Signature, SigningKey};
use serde_json::{Value, json};

use super::{
    MAX_PUBLIC_JWKS_BYTES, MAX_VALIDATOR_CONFIG_BYTES, PUBLIC_JWKS_PATH, RuntimeConfigError,
    VALIDATOR_CONFIG_PATH, ValidatorSnapshot, load_validator_snapshot_from_paths,
};
use crate::database_auth::{AuthMethod, DatabaseProfile, DatabaseTokenClaims, JwtValidationError};

const ISSUER: &str = "https://candidate.example.test/oauth/database";
const AUDIENCE: &str = "https://candidate.example.test/resources/database/gomtm-test";
const VALID_PUBLIC_JWKS: &str = r#"{"keys":[{"kty":"EC","crv":"P-256","alg":"ES256","use":"sig","key_ops":["verify"],"kid":"candidate-es256-pgx-gate","x":"HhhTL9R1TALzBB2cdc6zO4P_2BrHzk_ogsyxyYvFiW4","y":"pGwxHE4v9A3ZajZT5uRURdMt_khuztdcepDGoYiBwKM"}]}"#;
const NOW: i64 = 1_800_000_000;

static FIXTURE_ID: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    root: PathBuf,
    config: PathBuf,
    jwks: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let id = FIXTURE_ID.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "pggomtm-runtime-config-{}-{id}",
            std::process::id()
        ));
        fs::create_dir(&root).expect("create isolated runtime config fixture");
        Self {
            config: root.join("validator.json"),
            jwks: root.join("jwks.json"),
            root,
        }
    }

    fn write_valid(&self) {
        self.write_config(valid_config());
        self.write_jwks(VALID_PUBLIC_JWKS.as_bytes());
    }

    fn write_config(&self, value: Value) {
        let encoded = serde_json::to_vec(&value).expect("serialize config fixture");
        write_read_only(&self.config, &encoded);
    }

    fn write_config_bytes(&self, value: &[u8]) {
        write_read_only(&self.config, value);
    }

    fn write_jwks(&self, value: &[u8]) {
        write_read_only(&self.jwks, value);
    }

    fn load(&self) -> Result<ValidatorSnapshot, RuntimeConfigError> {
        load_validator_snapshot_from_paths(&self.config, &self.jwks)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).expect("remove isolated runtime config fixture");
    }
}

#[test]
fn runtime_paths_are_fixed_by_the_v1_contract() {
    assert_eq!(VALIDATOR_CONFIG_PATH, "/etc/pggomtm/validator.json");
    assert_eq!(PUBLIC_JWKS_PATH, "/etc/pggomtm/jwks.json");
}

#[test]
fn valid_read_only_config_and_public_jwks_create_a_snapshot() {
    let fixture = Fixture::new();
    fixture.write_valid();

    fixture.load().expect("valid immutable verifier snapshot");
}

#[test]
fn materials_in_different_directories_are_rejected() {
    let config_fixture = Fixture::new();
    let jwks_fixture = Fixture::new();
    config_fixture.write_config(valid_config());
    jwks_fixture.write_jwks(VALID_PUBLIC_JWKS.as_bytes());

    assert_eq!(
        load_validator_snapshot_from_paths(&config_fixture.config, &jwks_fixture.jwks)
            .expect_err("config and JWKS outside one directory must fail closed"),
        RuntimeConfigError::UnsafePublicationLayout
    );
}

#[test]
fn material_directory_symlink_is_rejected() {
    let fixture = Fixture::new();
    let real_directory = fixture.root.join("real");
    let linked_directory = fixture.root.join("linked");
    fs::create_dir(&real_directory).expect("create real material directory");
    write_read_only(
        &real_directory.join("validator.json"),
        &serde_json::to_vec(&valid_config()).expect("serialize config fixture"),
    );
    write_read_only(
        &real_directory.join("jwks.json"),
        VALID_PUBLIC_JWKS.as_bytes(),
    );
    symlink(&real_directory, &linked_directory).expect("link material directory");

    assert_eq!(
        load_validator_snapshot_from_paths(
            &linked_directory.join("validator.json"),
            &linked_directory.join("jwks.json"),
        )
        .expect_err("a symlinked publication directory must fail closed"),
        RuntimeConfigError::UnsafePublicationLayout
    );
}

#[test]
fn atomic_jwks_rotation_isolated_existing_and_later_backend_snapshots() {
    let fixture = Fixture::new();
    let retiring_key = signing_key(7);
    let active_key = signing_key(9);
    let retiring_kid = "candidate-es256-retiring";
    let active_kid = "candidate-es256-active";
    let retiring_token = signed_token_with_key(retiring_kid, &retiring_key);
    let active_token = signed_token_with_key(active_kid, &active_key);

    fixture.write_config(valid_config());
    fixture.write_jwks(
        &serde_json::to_vec(&jwks_with(vec![jwk_value(&retiring_key, retiring_kid)]))
            .expect("serialize retiring-key JWKS"),
    );
    let existing_backend = fixture.load().expect("existing backend snapshot");
    assert_snapshot_accepts(&existing_backend, &retiring_token);
    assert_snapshot_rejects_unknown_key(&existing_backend, &active_token);

    let staged_jwks = fixture.root.join(".jwks.json.next");
    write_read_only(&staged_jwks, br#"{"keys":["#);
    let backend_while_publisher_is_writing = fixture
        .load()
        .expect("active name must still expose the complete retiring-key snapshot");
    assert_snapshot_accepts(&backend_while_publisher_is_writing, &retiring_token);
    assert_snapshot_rejects_unknown_key(&backend_while_publisher_is_writing, &active_token);

    let active_and_retiring = jwks_with(vec![
        jwk_value(&active_key, active_kid),
        jwk_value(&retiring_key, retiring_kid),
    ]);
    write_read_only(
        &staged_jwks,
        &serde_json::to_vec(&active_and_retiring).expect("serialize rotating JWKS"),
    );
    let old_metadata = fs::metadata(&fixture.jwks).expect("old active JWKS metadata");
    let staged_metadata = fs::metadata(&staged_jwks).expect("staged JWKS metadata");
    assert_eq!(old_metadata.dev(), staged_metadata.dev());
    fs::rename(&staged_jwks, &fixture.jwks).expect("atomically publish rotating JWKS");
    let rotating_metadata = fs::metadata(&fixture.jwks).expect("rotating JWKS metadata");
    assert_eq!(old_metadata.dev(), rotating_metadata.dev());
    assert_ne!(old_metadata.ino(), rotating_metadata.ino());

    let later_backend = fixture.load().expect("later backend rotating snapshot");
    assert_snapshot_accepts(&later_backend, &active_token);
    assert_snapshot_accepts(&later_backend, &retiring_token);
    assert_snapshot_rejects_unknown_key(&existing_backend, &active_token);

    write_read_only(
        &staged_jwks,
        &serde_json::to_vec(&jwks_with(vec![jwk_value(&active_key, active_kid)]))
            .expect("serialize active-only JWKS"),
    );
    fs::rename(&staged_jwks, &fixture.jwks).expect("atomically retire old key");

    let newest_backend = fixture.load().expect("newest backend active snapshot");
    assert_snapshot_accepts(&newest_backend, &active_token);
    assert_snapshot_rejects_unknown_key(&newest_backend, &retiring_token);
    assert_snapshot_accepts(&existing_backend, &retiring_token);
    assert_snapshot_accepts(&later_backend, &retiring_token);
}

#[test]
fn missing_config_is_rejected() {
    let fixture = Fixture::new();
    fixture.write_jwks(VALID_PUBLIC_JWKS.as_bytes());

    assert_load_error(&fixture, RuntimeConfigError::ConfigMissing);
}

#[test]
fn missing_jwks_is_rejected() {
    let fixture = Fixture::new();
    fixture.write_config(valid_config());

    assert_load_error(&fixture, RuntimeConfigError::JwksMissing);
}

#[test]
fn oversized_config_is_rejected() {
    let fixture = Fixture::new();
    fixture.write_config_bytes(&vec![b' '; MAX_VALIDATOR_CONFIG_BYTES + 1]);
    fixture.write_jwks(VALID_PUBLIC_JWKS.as_bytes());

    assert_load_error(&fixture, RuntimeConfigError::ConfigTooLarge);
}

#[test]
fn oversized_jwks_is_rejected() {
    let fixture = Fixture::new();
    fixture.write_config(valid_config());
    fixture.write_jwks(&vec![b' '; MAX_PUBLIC_JWKS_BYTES + 1]);

    assert_load_error(&fixture, RuntimeConfigError::JwksTooLarge);
}

#[test]
fn writable_config_is_rejected() {
    let fixture = Fixture::new();
    fixture.write_valid();
    fs::set_permissions(&fixture.config, fs::Permissions::from_mode(0o644))
        .expect("make config writable");

    assert_load_error(&fixture, RuntimeConfigError::UnsafePermissions);
}

#[test]
fn writable_jwks_is_rejected() {
    let fixture = Fixture::new();
    fixture.write_valid();
    fs::set_permissions(&fixture.jwks, fs::Permissions::from_mode(0o644))
        .expect("make JWKS writable");

    assert_load_error(&fixture, RuntimeConfigError::UnsafePermissions);
}

#[test]
fn symlinked_material_is_rejected() {
    let fixture = Fixture::new();
    fixture.write_valid();
    let target = fixture.root.join("jwks-target.json");
    write_read_only(&target, VALID_PUBLIC_JWKS.as_bytes());
    fs::remove_file(&fixture.jwks).expect("remove real JWKS fixture");
    symlink(&target, &fixture.jwks).expect("create JWKS symlink");

    assert_load_error(&fixture, RuntimeConfigError::UnsafeFileType);
}

#[test]
fn non_v1_or_unknown_config_fields_are_rejected() {
    for (field, value) in [
        ("schema", json!("pggomtm-validator-config/v2")),
        ("algorithm", json!("ES256")),
        ("scope", json!("database")),
        ("max_ttl", json!(300)),
        ("role_mapping", json!({ "ordinary": "postgres" })),
        ("fallback_issuer", json!("https://fallback.example.test")),
        ("private_key_path", json!("/run/secrets/signing-key.pem")),
    ] {
        let fixture = Fixture::new();
        let mut config = valid_config();
        config
            .as_object_mut()
            .expect("config object")
            .insert(field.into(), value);
        fixture.write_config(config);
        fixture.write_jwks(VALID_PUBLIC_JWKS.as_bytes());

        assert_eq!(
            fixture.load().expect_err("unknown field must be rejected"),
            RuntimeConfigError::InvalidConfig,
            "field {field} must fail closed"
        );
    }
}

#[test]
fn invalid_or_ambiguous_https_resources_are_rejected() {
    for (issuer, audience) in [
        ("http://candidate.example.test/issuer", AUDIENCE),
        ("https://user@candidate.example.test/issuer", AUDIENCE),
        ("https://candidate.example.test/issuer?tenant=1", AUDIENCE),
        (ISSUER, "https://candidate.example.test/database#fragment"),
        (ISSUER, ISSUER),
    ] {
        let fixture = Fixture::new();
        let mut config = valid_config();
        config["issuer"] = json!(issuer);
        config["audience"] = json!(audience);
        fixture.write_config(config);
        fixture.write_jwks(VALID_PUBLIC_JWKS.as_bytes());

        assert_eq!(
            fixture.load().expect_err("resource must be rejected"),
            RuntimeConfigError::InvalidResources,
            "issuer={issuer} audience={audience} must fail closed"
        );
    }
}

#[test]
fn noncanonical_jwks_path_is_rejected() {
    let fixture = Fixture::new();
    let mut config = valid_config();
    config["jwks_path"] = json!("https://candidate.example.test/jwks.json");
    fixture.write_config(config);
    fixture.write_jwks(VALID_PUBLIC_JWKS.as_bytes());

    assert_load_error(&fixture, RuntimeConfigError::InvalidConfig);
}

#[test]
fn empty_jwks_file_is_rejected() {
    let fixture = Fixture::new();
    fixture.write_config(valid_config());
    fixture.write_jwks(b"");

    assert_load_error(&fixture, RuntimeConfigError::InvalidJwks);
}

#[test]
fn empty_jwks_key_set_is_rejected() {
    assert_jwks_rejected(json!({ "keys": [] }), RuntimeConfigError::InvalidJwks);
}

#[test]
fn empty_jwks_key_id_is_rejected() {
    let mut jwks: Value = serde_json::from_str(VALID_PUBLIC_JWKS).expect("public JWKS fixture");
    jwks["keys"][0]["kid"] = json!("");

    assert_jwks_rejected(jwks, RuntimeConfigError::InvalidJwks);
}

#[test]
fn duplicate_jwks_key_id_is_rejected() {
    let mut jwks: Value = serde_json::from_str(VALID_PUBLIC_JWKS).expect("public JWKS fixture");
    let key = jwks["keys"][0].clone();
    jwks["keys"].as_array_mut().expect("keys array").push(key);

    assert_jwks_rejected(jwks, RuntimeConfigError::DuplicateKeyId);
}

#[test]
fn private_jwk_is_rejected() {
    let mut jwks: Value = serde_json::from_str(VALID_PUBLIC_JWKS).expect("public JWKS fixture");
    jwks["keys"][0]["d"] = json!("private-material-must-not-load");

    assert_jwks_rejected(jwks, RuntimeConfigError::InvalidJwks);
}

#[test]
fn non_es256_jwk_is_rejected() {
    let mut jwks: Value = serde_json::from_str(VALID_PUBLIC_JWKS).expect("public JWKS fixture");
    jwks["keys"][0]["alg"] = json!("ES384");

    assert_jwks_rejected(jwks, RuntimeConfigError::InvalidJwks);
}

#[test]
fn token_with_unknown_kid_is_rejected_by_the_snapshot() {
    let fixture = Fixture::new();
    fixture.write_valid();
    let snapshot = fixture.load().expect("valid snapshot");
    let token = signed_token("unknown-kid");

    assert_eq!(
        snapshot.verify(&token, DatabaseProfile::Ordinary.database_role(), NOW + 1),
        Err(JwtValidationError::UnknownKeyId)
    );
}

fn valid_config() -> Value {
    json!({
        "schema": "pggomtm-validator-config/v1",
        "issuer": ISSUER,
        "audience": AUDIENCE,
        "jwks_path": PUBLIC_JWKS_PATH,
    })
}

fn assert_jwks_rejected(jwks: Value, expected: RuntimeConfigError) {
    let fixture = Fixture::new();
    fixture.write_config(valid_config());
    fixture.write_jwks(&serde_json::to_vec(&jwks).expect("serialize rejected public JWKS fixture"));
    assert_load_error(&fixture, expected);
}

fn assert_load_error(fixture: &Fixture, expected: RuntimeConfigError) {
    assert_eq!(
        fixture
            .load()
            .expect_err("invalid runtime material must fail closed"),
        expected
    );
}

fn signed_token(kid: &str) -> String {
    signed_token_with_key(kid, &signing_key(7))
}

fn signing_key(byte: u8) -> SigningKey {
    SigningKey::from_slice(&[byte; 32]).expect("fixed synthetic signing fixture")
}

fn jwk_value(key: &SigningKey, kid: &str) -> Value {
    let mut value =
        serde_json::to_value(JsonWebKey::build(key.verifying_key())).expect("serialize public JWK");
    let object = value.as_object_mut().expect("JWK object");
    object.insert("alg".into(), json!("ES256"));
    object.insert("key_ops".into(), json!(["verify"]));
    object.insert("kid".into(), json!(kid));
    object.insert("use".into(), json!("sig"));
    value
}

fn jwks_with(keys: Vec<Value>) -> Value {
    json!({ "keys": keys })
}

fn signed_token_with_key(kid: &str, key: &SigningKey) -> String {
    let claims = DatabaseTokenClaims {
        issuer: ISSUER.into(),
        audience: AUDIENCE.into(),
        subject: "usr_snapshot_gate".into(),
        issued_at: NOW,
        expires_at: NOW + 120,
        token_id: "jti_snapshot_gate".into(),
        scope: "database".into(),
        delegation_id: "dlg_snapshot_gate".into(),
        auth_method: AuthMethod::OAuth,
        authority_version: 1,
        db_profile: DatabaseProfile::Ordinary,
        db_role: DatabaseProfile::Ordinary.database_role().into(),
        client_id: Some("cli_snapshot_gate".into()),
        credential_id: None,
    };
    let mut token = Token::compact((), claims);
    *token.header_mut().key_id() = Some(kid.into());
    token
        .sign::<_, Signature>(key)
        .expect("sign unknown-kid fixture")
        .rendered()
        .expect("render unknown-kid fixture")
}

fn assert_snapshot_accepts(snapshot: &ValidatorSnapshot, token: &str) {
    snapshot
        .verify(token, DatabaseProfile::Ordinary.database_role(), NOW + 1)
        .expect("snapshot must accept the expected key");
}

fn assert_snapshot_rejects_unknown_key(snapshot: &ValidatorSnapshot, token: &str) {
    assert_eq!(
        snapshot.verify(token, DatabaseProfile::Ordinary.database_role(), NOW + 1),
        Err(JwtValidationError::UnknownKeyId)
    );
}

fn write_read_only(path: &Path, value: &[u8]) {
    if fs::symlink_metadata(path).is_ok() {
        fs::remove_file(path).expect("replace fixture file");
    }
    fs::write(path, value).expect("write fixture file");
    fs::set_permissions(path, fs::Permissions::from_mode(0o444)).expect("make fixture read-only");
}
