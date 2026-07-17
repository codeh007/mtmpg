use std::fmt;
use std::path::Path;

use crate::database_auth::{DatabaseTokenVerifier, JwtValidationError, VerifiedDatabaseToken};

pub const VALIDATOR_CONFIG_PATH: &str = "/etc/pggomtm/validator.json";
pub const PUBLIC_JWKS_PATH: &str = "/etc/pggomtm/jwks.json";
pub const MAX_VALIDATOR_CONFIG_BYTES: usize = 16_384;
pub const MAX_PUBLIC_JWKS_BYTES: usize = 65_536;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeConfigError {
    Unavailable,
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
        write!(formatter, "runtime config error: {self:?}")
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
    _config_path: &Path,
    _jwks_path: &Path,
) -> Result<ValidatorSnapshot, RuntimeConfigError> {
    Err(RuntimeConfigError::Unavailable)
}

#[cfg(test)]
mod tests;
