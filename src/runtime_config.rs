use std::fmt;
use std::fs::{self, File};
use std::io::Read;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;

use serde::Deserialize;

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
            Self::InvalidConfig => "validator config is invalid",
            Self::InvalidResources => "validator issuer or audience is invalid",
            Self::InvalidJwks => "public JWKS is invalid",
            Self::DuplicateKeyId => "public JWKS contains a duplicate key ID",
        })
    }
}

impl std::error::Error for RuntimeConfigError {}

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
    let config_bytes = read_immutable_file(
        config_path,
        MAX_VALIDATOR_CONFIG_BYTES,
        RuntimeConfigError::ConfigMissing,
        RuntimeConfigError::ConfigTooLarge,
        RuntimeConfigError::InvalidConfig,
    )?;
    if config_bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return Err(RuntimeConfigError::InvalidConfig);
    }
    let config: ValidatorConfigDocument =
        serde_json::from_slice(&config_bytes).map_err(|_| RuntimeConfigError::InvalidConfig)?;
    if config.schema != VALIDATOR_CONFIG_SCHEMA || config.jwks_path != PUBLIC_JWKS_PATH {
        return Err(RuntimeConfigError::InvalidConfig);
    }
    let policy = DatabaseTokenPolicy::new(config.issuer, config.audience)
        .map_err(|_| RuntimeConfigError::InvalidResources)?;

    let jwks_bytes = read_immutable_file(
        jwks_path,
        MAX_PUBLIC_JWKS_BYTES,
        RuntimeConfigError::JwksMissing,
        RuntimeConfigError::JwksTooLarge,
        RuntimeConfigError::InvalidJwks,
    )?;
    let jwks = std::str::from_utf8(&jwks_bytes).map_err(|_| RuntimeConfigError::InvalidJwks)?;
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

fn read_immutable_file(
    path: &Path,
    maximum_bytes: usize,
    missing: RuntimeConfigError,
    too_large: RuntimeConfigError,
    invalid: RuntimeConfigError,
) -> Result<Vec<u8>, RuntimeConfigError> {
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
    Ok(bytes)
}

#[cfg(test)]
mod tests;
