use std::fmt;

use jaws::Token;
use p256::ecdsa::{Signature, SigningKey};
use pggomtm::database_auth::{DatabaseTokenClaims, DatabaseTokenPolicy};
use uuid::Uuid;
use zeroize::Zeroizing;

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
    issuer: String,
    audience: String,
    key_id: String,
}

impl IssuerConfig {
    pub fn new(
        issuer: impl Into<String>,
        audience: impl Into<String>,
        key_id: impl Into<String>,
    ) -> Result<Self, IssuerError> {
        let issuer = issuer.into();
        let audience = audience.into();
        let key_id = key_id.into();
        if DatabaseTokenPolicy::new(issuer.clone(), audience.clone()).is_err()
            || !is_valid_key_id(&key_id)
        {
            return Err(IssuerError::InvalidConfiguration);
        }
        Ok(Self {
            issuer,
            audience,
            key_id,
        })
    }
}

pub struct DatabaseTokenIssuer {
    config: IssuerConfig,
    signing_key: SigningKey,
}

impl DatabaseTokenIssuer {
    #[must_use]
    pub fn new(config: IssuerConfig, signing_key: SigningKey) -> Self {
        Self {
            config,
            signing_key,
        }
    }

    pub fn issue(
        &self,
        principal: &DelegatedPrincipal,
        now: i64,
    ) -> Result<IssuedDatabaseToken, IssuerError> {
        if now < 0 || !principal.is_valid() {
            return Err(IssuerError::InvalidPrincipal);
        }
        let expires_at = now
            .checked_add(DATABASE_TOKEN_TTL_SECONDS)
            .ok_or(IssuerError::InvalidPrincipal)?;
        if principal.credential_expires_at < expires_at {
            return Err(IssuerError::CredentialExpiresTooSoon);
        }

        let (client_id, credential_id) = match principal.auth_method {
            pggomtm::database_auth::AuthMethod::OAuth => (principal.client_id.clone(), None),
            pggomtm::database_auth::AuthMethod::ApiKey => (None, principal.credential_id.clone()),
        };
        let claims = DatabaseTokenClaims {
            issuer: self.config.issuer.clone(),
            audience: self.config.audience.clone(),
            subject: principal.user_id.clone(),
            issued_at: now,
            expires_at,
            token_id: Uuid::new_v4().simple().to_string(),
            scope: "database".into(),
            delegation_id: principal.delegation_id.clone(),
            auth_method: principal.auth_method,
            authority_version: principal.authority_version,
            db_profile: principal.profile,
            db_role: principal.profile.database_role().into(),
            client_id,
            credential_id,
        };
        let mut token = Token::compact((), claims);
        *token.header_mut().key_id() = Some(self.config.key_id.clone());
        let encoded = token
            .sign::<_, Signature>(&self.signing_key)
            .map_err(|_| IssuerError::SigningFailed)?
            .rendered()
            .map_err(|_| IssuerError::SigningFailed)?;
        Ok(IssuedDatabaseToken(Zeroizing::new(encoded)))
    }
}

#[derive(PartialEq, Eq)]
pub struct IssuedDatabaseToken(Zeroizing<String>);

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

fn is_valid_key_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
}
