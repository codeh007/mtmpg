use std::fmt;

use p256::ecdsa::SigningKey;

use crate::protocol::DelegatedPrincipal;

pub const DATABASE_TOKEN_TTL_SECONDS: i64 = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssuerError {
    InvalidConfiguration,
    InvalidPrincipal,
    CredentialExpiresTooSoon,
    SigningFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssuerConfig {
    _issuer: String,
    _audience: String,
    _key_id: String,
}

impl IssuerConfig {
    pub fn new(
        _issuer: impl Into<String>,
        _audience: impl Into<String>,
        _key_id: impl Into<String>,
    ) -> Result<Self, IssuerError> {
        todo!("issuer configuration is implemented after RED verification")
    }
}

pub struct DatabaseTokenIssuer {
    _config: IssuerConfig,
    _signing_key: SigningKey,
}

impl DatabaseTokenIssuer {
    #[must_use]
    pub fn new(config: IssuerConfig, signing_key: SigningKey) -> Self {
        Self {
            _config: config,
            _signing_key: signing_key,
        }
    }

    pub fn issue(
        &self,
        _principal: &DelegatedPrincipal,
        _now: i64,
    ) -> Result<IssuedDatabaseToken, IssuerError> {
        todo!("database token issuance is implemented after RED verification")
    }
}

#[derive(PartialEq, Eq)]
pub struct IssuedDatabaseToken(String);

impl IssuedDatabaseToken {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for IssuedDatabaseToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("IssuedDatabaseToken([REDACTED])")
    }
}
