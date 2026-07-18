use std::fmt;
use std::fs::{self, File};
use std::io::Read;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;

use serde::Deserialize;

use crate::auth_failure::AuthenticationFailureReason;
use crate::database_auth::{
    DatabaseTokenPolicy, DatabaseTokenVerifier, JwtValidationError, MAX_JWKS_BYTES,
    VerifiedDatabaseToken,
};

pub const VALIDATOR_CONFIG_PATH: &str = "/etc/pggomtm/validator.json";
pub const PUBLIC_JWKS_PATH: &str = "/etc/pggomtm/jwks.json";
pub const MAX_VALIDATOR_CONFIG_BYTES: usize = 16_384;
pub const MAX_PUBLIC_JWKS_BYTES: usize = MAX_JWKS_BYTES;
const VALIDATOR_CONFIG_SCHEMA: &str = "pggomtm-validator-config/v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeConfigError {
    ConfigMissing,
    JwksMissing,
    ConfigTooLarge,
    JwksTooLarge,
    UnsafeFileType,
    UnsafePermissions,
    UnsafePublicationLayout,
    InvalidConfig,
    InvalidResources,
    InvalidJwks,
    DuplicateKeyId,
}

impl fmt::Display for RuntimeConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ConfigMissing => "validator config is missing",
            Self::JwksMissing => "public JWKS is missing",
            Self::ConfigTooLarge => "validator config exceeds its size limit",
            Self::JwksTooLarge => "public JWKS exceeds its size limit",
            Self::UnsafeFileType => "validator material is not a regular file",
            Self::UnsafePermissions => "validator material is writable",
            Self::UnsafePublicationLayout => {
                "validator materials do not share a safe atomic publication directory"
            }
            Self::InvalidConfig => "validator config is invalid",
            Self::InvalidResources => "validator issuer or audience is invalid",
            Self::InvalidJwks => "public JWKS is invalid",
            Self::DuplicateKeyId => "public JWKS contains a duplicate key ID",
        })
    }
}

impl std::error::Error for RuntimeConfigError {}

impl RuntimeConfigError {
    #[must_use]
    pub const fn reason(self) -> AuthenticationFailureReason {
        match self {
            Self::ConfigMissing => AuthenticationFailureReason::ConfigMissing,
            Self::JwksMissing => AuthenticationFailureReason::JwksMissing,
            Self::ConfigTooLarge => AuthenticationFailureReason::ConfigTooLarge,
            Self::JwksTooLarge => AuthenticationFailureReason::JwksTooLarge,
            Self::UnsafeFileType => AuthenticationFailureReason::UnsafeFileType,
            Self::UnsafePermissions => AuthenticationFailureReason::UnsafePermissions,
            Self::UnsafePublicationLayout => AuthenticationFailureReason::UnsafePublicationLayout,
            Self::InvalidConfig => AuthenticationFailureReason::InvalidConfig,
            Self::InvalidResources => AuthenticationFailureReason::InvalidResources,
            Self::InvalidJwks => AuthenticationFailureReason::InvalidJwks,
            Self::DuplicateKeyId => AuthenticationFailureReason::DuplicateKeyId,
        }
    }

    #[must_use]
    pub const fn reason_code(self) -> &'static str {
        self.reason().code()
    }
}

#[derive(Debug)]
pub struct ValidatorSnapshot {
    verifier: DatabaseTokenVerifier,
}

impl ValidatorSnapshot {
    pub fn verify(
        &self,
        compact_token: &str,
        requested_role: &str,
        now: i64,
    ) -> Result<VerifiedDatabaseToken, JwtValidationError> {
        self.verifier.verify(compact_token, requested_role, now)
    }
}

pub fn load_validator_snapshot() -> Result<ValidatorSnapshot, RuntimeConfigError> {
    load_validator_snapshot_from_paths(
        Path::new(VALIDATOR_CONFIG_PATH),
        Path::new(PUBLIC_JWKS_PATH),
    )
}

fn load_validator_snapshot_from_paths(
    config_path: &Path,
    jwks_path: &Path,
) -> Result<ValidatorSnapshot, RuntimeConfigError> {
    let config_material = read_immutable_file(
        config_path,
        MAX_VALIDATOR_CONFIG_BYTES,
        RuntimeConfigError::ConfigMissing,
        RuntimeConfigError::ConfigTooLarge,
        RuntimeConfigError::InvalidConfig,
    )?;
    let config_bytes = config_material.bytes.as_slice();
    if config_bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return Err(RuntimeConfigError::InvalidConfig);
    }
    let config: ValidatorConfigDocument =
        serde_json::from_slice(config_bytes).map_err(|_| RuntimeConfigError::InvalidConfig)?;
    if config.schema != VALIDATOR_CONFIG_SCHEMA || config.jwks_path != PUBLIC_JWKS_PATH {
        return Err(RuntimeConfigError::InvalidConfig);
    }
    let policy = DatabaseTokenPolicy::new(config.issuer, config.audience)
        .map_err(|_| RuntimeConfigError::InvalidResources)?;

    let jwks_material = read_immutable_file(
        jwks_path,
        MAX_PUBLIC_JWKS_BYTES,
        RuntimeConfigError::JwksMissing,
        RuntimeConfigError::JwksTooLarge,
        RuntimeConfigError::InvalidJwks,
    )?;
    validate_atomic_publication_layout(
        config_path,
        config_material.device,
        jwks_path,
        jwks_material.device,
    )?;
    let jwks_bytes = jwks_material.bytes.as_slice();
    let jwks = std::str::from_utf8(jwks_bytes).map_err(|_| RuntimeConfigError::InvalidJwks)?;
    let verifier = DatabaseTokenVerifier::from_jwks(jwks, policy).map_err(|error| match error {
        JwtValidationError::DuplicateKeyId => RuntimeConfigError::DuplicateKeyId,
        _ => RuntimeConfigError::InvalidJwks,
    })?;

    Ok(ValidatorSnapshot { verifier })
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ValidatorConfigDocument {
    schema: String,
    issuer: String,
    audience: String,
    jwks_path: String,
}

struct ImmutableMaterial {
    bytes: Vec<u8>,
    device: u64,
}

fn validate_atomic_publication_layout(
    config_path: &Path,
    config_device: u64,
    jwks_path: &Path,
    jwks_device: u64,
) -> Result<(), RuntimeConfigError> {
    let Some(publication_directory) = config_path.parent() else {
        return Err(RuntimeConfigError::UnsafePublicationLayout);
    };
    if jwks_path.parent() != Some(publication_directory) {
        return Err(RuntimeConfigError::UnsafePublicationLayout);
    }

    let directory_metadata = fs::symlink_metadata(publication_directory)
        .map_err(|_| RuntimeConfigError::UnsafePublicationLayout)?;
    if !directory_metadata.file_type().is_dir()
        || directory_metadata.dev() != config_device
        || directory_metadata.dev() != jwks_device
    {
        return Err(RuntimeConfigError::UnsafePublicationLayout);
    }

    Ok(())
}

fn read_immutable_file(
    path: &Path,
    maximum_bytes: usize,
    missing: RuntimeConfigError,
    too_large: RuntimeConfigError,
    invalid: RuntimeConfigError,
) -> Result<ImmutableMaterial, RuntimeConfigError> {
    let path_metadata = fs::symlink_metadata(path).map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => missing,
        std::io::ErrorKind::PermissionDenied => RuntimeConfigError::UnsafePermissions,
        _ => invalid,
    })?;
    if !path_metadata.file_type().is_file() {
        return Err(RuntimeConfigError::UnsafeFileType);
    }

    let mut file = File::open(path).map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => missing,
        std::io::ErrorKind::PermissionDenied => RuntimeConfigError::UnsafePermissions,
        _ => invalid,
    })?;
    let opened_metadata = file.metadata().map_err(|_| invalid)?;
    if !opened_metadata.file_type().is_file()
        || opened_metadata.dev() != path_metadata.dev()
        || opened_metadata.ino() != path_metadata.ino()
    {
        return Err(RuntimeConfigError::UnsafeFileType);
    }
    if opened_metadata.permissions().mode() & 0o222 != 0 {
        return Err(RuntimeConfigError::UnsafePermissions);
    }
    if opened_metadata.len() > maximum_bytes as u64 {
        return Err(too_large);
    }

    let mut bytes = Vec::with_capacity(opened_metadata.len() as usize);
    file.by_ref()
        .take((maximum_bytes + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| invalid)?;
    if bytes.len() > maximum_bytes {
        return Err(too_large);
    }
    Ok(ImmutableMaterial {
        bytes,
        device: opened_metadata.dev(),
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::{PermissionsExt, symlink};
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
    use crate::database_auth::{
        AuthMethod, DatabaseProfile, DatabaseTokenClaims, JwtValidationError,
    };

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
            write_read_only(
                &self.config,
                &serde_json::to_vec(&value).expect("serialize config fixture"),
            );
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
    fn fixed_paths_and_valid_material_create_a_snapshot() {
        assert_eq!(VALIDATOR_CONFIG_PATH, "/etc/pggomtm/validator.json");
        assert_eq!(PUBLIC_JWKS_PATH, "/etc/pggomtm/jwks.json");

        let fixture = Fixture::new();
        fixture.write_valid();
        let snapshot = fixture.load().expect("valid immutable verifier snapshot");
        let unknown_key = signed_token_with_key("unknown-kid", &signing_key(9));
        assert_eq!(
            snapshot.verify(
                &unknown_key,
                DatabaseProfile::Ordinary.database_role(),
                NOW + 1
            ),
            Err(JwtValidationError::UnknownKeyId)
        );
    }

    #[test]
    fn unsafe_publication_layout_fails_closed() {
        let config_fixture = Fixture::new();
        let jwks_fixture = Fixture::new();
        config_fixture.write_config(valid_config());
        jwks_fixture.write_jwks(VALID_PUBLIC_JWKS.as_bytes());
        assert_eq!(
            load_validator_snapshot_from_paths(&config_fixture.config, &jwks_fixture.jwks)
                .expect_err("materials outside one directory must fail closed"),
            RuntimeConfigError::UnsafePublicationLayout
        );

        let fixture = Fixture::new();
        let real = fixture.root.join("real");
        let linked = fixture.root.join("linked");
        fs::create_dir(&real).expect("create real material directory");
        write_read_only(
            &real.join("validator.json"),
            &serde_json::to_vec(&valid_config()).expect("serialize config fixture"),
        );
        write_read_only(&real.join("jwks.json"), VALID_PUBLIC_JWKS.as_bytes());
        symlink(&real, &linked).expect("link material directory");
        assert_eq!(
            load_validator_snapshot_from_paths(
                &linked.join("validator.json"),
                &linked.join("jwks.json"),
            )
            .expect_err("symlinked publication directory must fail closed"),
            RuntimeConfigError::UnsafePublicationLayout
        );
    }

    #[test]
    fn missing_oversized_writable_and_symlinked_material_fail_closed() {
        assert_fixture_error(RuntimeConfigError::ConfigMissing, |fixture| {
            fixture.write_jwks(VALID_PUBLIC_JWKS.as_bytes());
        });
        assert_fixture_error(RuntimeConfigError::JwksMissing, |fixture| {
            fixture.write_config(valid_config());
        });
        assert_fixture_error(RuntimeConfigError::ConfigTooLarge, |fixture| {
            write_read_only(&fixture.config, &vec![b' '; MAX_VALIDATOR_CONFIG_BYTES + 1]);
            fixture.write_jwks(VALID_PUBLIC_JWKS.as_bytes());
        });
        assert_fixture_error(RuntimeConfigError::JwksTooLarge, |fixture| {
            fixture.write_config(valid_config());
            fixture.write_jwks(&vec![b' '; MAX_PUBLIC_JWKS_BYTES + 1]);
        });
        assert_fixture_error(RuntimeConfigError::UnsafePermissions, |fixture| {
            fixture.write_valid();
            fs::set_permissions(&fixture.config, fs::Permissions::from_mode(0o644))
                .expect("make config writable");
        });
        assert_fixture_error(RuntimeConfigError::UnsafePermissions, |fixture| {
            fixture.write_valid();
            fs::set_permissions(&fixture.jwks, fs::Permissions::from_mode(0o644))
                .expect("make JWKS writable");
        });
        assert_fixture_error(RuntimeConfigError::UnsafeFileType, |fixture| {
            fixture.write_config(valid_config());
            let target = fixture.root.join("jwks-target.json");
            write_read_only(&target, VALID_PUBLIC_JWKS.as_bytes());
            symlink(&target, &fixture.jwks).expect("create JWKS symlink");
        });
    }

    #[test]
    fn invalid_config_schema_resources_and_path_fail_closed() {
        let mut unknown_field = valid_config();
        unknown_field["algorithm"] = json!("ES256");
        let mut invalid_issuer = valid_config();
        invalid_issuer["issuer"] = json!("http://candidate.example.test/issuer");
        let mut same_resources = valid_config();
        same_resources["audience"] = json!(ISSUER);
        let mut invalid_path = valid_config();
        invalid_path["jwks_path"] = json!("https://candidate.example.test/jwks.json");

        for (config, expected) in [
            (
                json!({
                    "schema": "pggomtm-validator-config/v2",
                    "issuer": ISSUER,
                    "audience": AUDIENCE,
                    "jwks_path": PUBLIC_JWKS_PATH,
                }),
                RuntimeConfigError::InvalidConfig,
            ),
            (unknown_field, RuntimeConfigError::InvalidConfig),
            (invalid_issuer, RuntimeConfigError::InvalidResources),
            (same_resources, RuntimeConfigError::InvalidResources),
            (invalid_path, RuntimeConfigError::InvalidConfig),
        ] {
            let fixture = Fixture::new();
            fixture.write_config(config);
            fixture.write_jwks(VALID_PUBLIC_JWKS.as_bytes());
            assert_load_error(&fixture, expected);
        }
    }

    #[test]
    fn invalid_public_keys_fail_closed() {
        let empty = json!({ "keys": [] });
        let mut duplicate: Value =
            serde_json::from_str(VALID_PUBLIC_JWKS).expect("public JWKS fixture");
        let duplicate_key = duplicate["keys"][0].clone();
        duplicate["keys"]
            .as_array_mut()
            .expect("keys array")
            .push(duplicate_key);
        let mut private: Value =
            serde_json::from_str(VALID_PUBLIC_JWKS).expect("public JWKS fixture");
        private["keys"][0]["d"] = json!("private-material-must-not-load");
        let mut wrong_algorithm: Value =
            serde_json::from_str(VALID_PUBLIC_JWKS).expect("public JWKS fixture");
        wrong_algorithm["keys"][0]["alg"] = json!("ES384");

        for (jwks, expected) in [
            (empty, RuntimeConfigError::InvalidJwks),
            (duplicate, RuntimeConfigError::DuplicateKeyId),
            (private, RuntimeConfigError::InvalidJwks),
            (wrong_algorithm, RuntimeConfigError::InvalidJwks),
        ] {
            let fixture = Fixture::new();
            fixture.write_config(valid_config());
            fixture.write_jwks(&serde_json::to_vec(&jwks).expect("serialize JWKS fixture"));
            assert_load_error(&fixture, expected);
        }
    }

    #[test]
    fn atomic_jwks_rotation_isolates_existing_and_later_snapshots() {
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
        let existing = fixture.load().expect("existing backend snapshot");
        assert_snapshot_accepts(&existing, &retiring_token);
        assert_snapshot_rejects_unknown_key(&existing, &active_token);

        let staged = fixture.root.join(".jwks.json.next");
        write_read_only(
            &staged,
            &serde_json::to_vec(&jwks_with(vec![
                jwk_value(&active_key, active_kid),
                jwk_value(&retiring_key, retiring_kid),
            ]))
            .expect("serialize rotating JWKS"),
        );
        fs::rename(&staged, &fixture.jwks).expect("publish rotating JWKS");
        let later = fixture.load().expect("later backend snapshot");
        assert_snapshot_accepts(&later, &active_token);
        assert_snapshot_accepts(&later, &retiring_token);
        assert_snapshot_rejects_unknown_key(&existing, &active_token);

        write_read_only(
            &staged,
            &serde_json::to_vec(&jwks_with(vec![jwk_value(&active_key, active_kid)]))
                .expect("serialize active-only JWKS"),
        );
        fs::rename(&staged, &fixture.jwks).expect("retire old key");
        let newest = fixture.load().expect("newest backend snapshot");
        assert_snapshot_accepts(&newest, &active_token);
        assert_snapshot_rejects_unknown_key(&newest, &retiring_token);
        assert_snapshot_accepts(&existing, &retiring_token);
        assert_snapshot_accepts(&later, &retiring_token);
    }

    fn valid_config() -> Value {
        json!({
            "schema": "pggomtm-validator-config/v1",
            "issuer": ISSUER,
            "audience": AUDIENCE,
            "jwks_path": PUBLIC_JWKS_PATH,
        })
    }

    fn assert_fixture_error(expected: RuntimeConfigError, setup: impl FnOnce(&Fixture)) {
        let fixture = Fixture::new();
        setup(&fixture);
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

    fn signing_key(byte: u8) -> SigningKey {
        SigningKey::from_slice(&[byte; 32]).expect("fixed synthetic signing fixture")
    }

    fn jwk_value(key: &SigningKey, kid: &str) -> Value {
        let mut value = serde_json::to_value(JsonWebKey::build(key.verifying_key()))
            .expect("serialize public JWK");
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
            .expect("sign snapshot fixture")
            .rendered()
            .expect("render snapshot fixture")
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
        fs::set_permissions(path, fs::Permissions::from_mode(0o444))
            .expect("make fixture read-only");
    }
}
