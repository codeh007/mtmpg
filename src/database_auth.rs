use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

use jaws::algorithms::AlgorithmIdentifier;
use jaws::key::DeserializeJWK;
use jaws::token::{TokenVerifyingError, Unverified};
use jaws::{Compact, Token};
use p256::ecdsa::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;

use crate::auth_failure::AuthenticationFailureReason;

pub const MIN_TOKEN_TTL_SECONDS: i64 = 30;
pub const MAX_TOKEN_TTL_SECONDS: i64 = 300;
pub const MAX_AUTHN_ID_BYTES: usize = 512;

const MAX_TOKEN_BYTES: usize = 8_192;
pub const MAX_JWKS_BYTES: usize = 65_536;
pub const MAX_JWKS_KEYS: usize = 16;
const MAX_INTERNAL_ID_BYTES: usize = 64;
const MAX_KEY_ID_BYTES: usize = 128;
const DATABASE_SCOPE: &str = "database";
const SYSTEM_USER_PREFIX: &str = "oauth:";
const AUTHN_ID_PREFIX: &str = "pggomtm:v2";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JwtValidationError {
    InvalidPolicy,
    InvalidJwks,
    DuplicateKeyId,
    InvalidToken,
    InvalidHeader,
    UnknownKeyId,
    InvalidSignature,
    InvalidClaims,
    RequestedRoleMismatch,
    InvalidIdentity,
}

impl fmt::Display for JwtValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidPolicy => "invalid database token policy",
            Self::InvalidJwks => "invalid database token JWKS",
            Self::DuplicateKeyId => "duplicate database token key ID",
            Self::InvalidToken => "invalid database token",
            Self::InvalidHeader => "invalid database token header",
            Self::UnknownKeyId => "unknown database token key ID",
            Self::InvalidSignature => "invalid database token signature",
            Self::InvalidClaims => "invalid database token claims",
            Self::RequestedRoleMismatch => "database token role mismatch",
            Self::InvalidIdentity => "invalid authenticated identity",
        })
    }
}

impl std::error::Error for JwtValidationError {}

impl JwtValidationError {
    #[must_use]
    pub const fn reason(self) -> AuthenticationFailureReason {
        match self {
            Self::InvalidPolicy => AuthenticationFailureReason::InvalidTokenPolicy,
            Self::InvalidJwks => AuthenticationFailureReason::InvalidJwks,
            Self::DuplicateKeyId => AuthenticationFailureReason::DuplicateKeyId,
            Self::InvalidToken => AuthenticationFailureReason::InvalidToken,
            Self::InvalidHeader => AuthenticationFailureReason::InvalidTokenHeader,
            Self::UnknownKeyId => AuthenticationFailureReason::UnknownKeyId,
            Self::InvalidSignature => AuthenticationFailureReason::InvalidSignature,
            Self::InvalidClaims => AuthenticationFailureReason::InvalidClaims,
            Self::RequestedRoleMismatch => AuthenticationFailureReason::RequestedRoleMismatch,
            Self::InvalidIdentity => AuthenticationFailureReason::InvalidIdentity,
        }
    }

    #[must_use]
    pub const fn reason_code(self) -> &'static str {
        self.reason().code()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatabaseTokenPolicy {
    issuer: String,
    audience: String,
}

impl DatabaseTokenPolicy {
    pub fn new(
        issuer: impl Into<String>,
        audience: impl Into<String>,
    ) -> Result<Self, JwtValidationError> {
        let issuer = issuer.into();
        let audience = audience.into();

        if issuer == audience
            || !is_strict_https_resource(&issuer)
            || !is_strict_https_resource(&audience)
        {
            return Err(JwtValidationError::InvalidPolicy);
        }

        Ok(Self { issuer, audience })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum AuthMethod {
    #[serde(rename = "oauth")]
    OAuth,
    #[serde(rename = "api_key")]
    ApiKey,
}

impl AuthMethod {
    const fn as_str(self) -> &'static str {
        match self {
            Self::OAuth => "oauth",
            Self::ApiKey => "api_key",
        }
    }
}

impl FromStr for AuthMethod {
    type Err = JwtValidationError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "oauth" => Ok(Self::OAuth),
            "api_key" => Ok(Self::ApiKey),
            _ => Err(JwtValidationError::InvalidIdentity),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DatabaseProfile {
    #[serde(rename = "ordinary")]
    Ordinary,
    #[serde(rename = "business_admin")]
    BusinessAdmin,
    #[serde(rename = "database_developer")]
    DatabaseDeveloper,
}

impl DatabaseProfile {
    #[must_use]
    pub const fn database_role(self) -> &'static str {
        match self {
            Self::Ordinary => "ordinary",
            Self::BusinessAdmin => "business_admin",
            Self::DatabaseDeveloper => "database_developer",
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::Ordinary => "ordinary",
            Self::BusinessAdmin => "business_admin",
            Self::DatabaseDeveloper => "database_developer",
        }
    }
}

impl FromStr for DatabaseProfile {
    type Err = JwtValidationError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "ordinary" => Ok(Self::Ordinary),
            "business_admin" => Ok(Self::BusinessAdmin),
            "database_developer" => Ok(Self::DatabaseDeveloper),
            _ => Err(JwtValidationError::InvalidIdentity),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthenticatedActor {
    OAuthClient(String),
    ApiKeyCredential(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedIdentity {
    pub user_id: String,
    pub actor: AuthenticatedActor,
    pub delegation_id: String,
    pub auth_method: AuthMethod,
    pub authority_version: u64,
    pub profile: DatabaseProfile,
}

impl AuthenticatedIdentity {
    pub fn encode_authn_id(&self) -> Result<String, JwtValidationError> {
        validate_identity(self)?;
        let (actor_kind, actor_id) = match &self.actor {
            AuthenticatedActor::OAuthClient(id) => ("client", id.as_str()),
            AuthenticatedActor::ApiKeyCredential(id) => ("credential", id.as_str()),
        };
        let encoded = format!(
            "{AUTHN_ID_PREFIX};u={};actor={actor_kind}:{actor_id};d={};m={};a={};p={}",
            self.user_id,
            self.delegation_id,
            self.auth_method.as_str(),
            self.authority_version,
            self.profile.as_str(),
        );

        if encoded.len() > MAX_AUTHN_ID_BYTES {
            return Err(JwtValidationError::InvalidIdentity);
        }

        Ok(encoded)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DatabaseTokenClaims {
    #[serde(rename = "iss")]
    pub issuer: String,
    #[serde(rename = "aud")]
    pub audience: String,
    #[serde(rename = "sub")]
    pub subject: String,
    #[serde(rename = "iat")]
    pub issued_at: i64,
    #[serde(rename = "exp")]
    pub expires_at: i64,
    #[serde(rename = "jti")]
    pub token_id: String,
    pub scope: String,
    pub delegation_id: String,
    pub auth_method: AuthMethod,
    pub authority_version: u64,
    pub db_profile: DatabaseProfile,
    pub db_role: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present_actor_id"
    )]
    pub client_id: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present_actor_id"
    )]
    pub credential_id: Option<String>,
}

fn deserialize_present_actor_id<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    String::deserialize(deserializer).map(Some)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedDatabaseToken {
    pub claims: DatabaseTokenClaims,
    pub identity: AuthenticatedIdentity,
    pub authn_id: String,
}

#[derive(Debug)]
pub struct DatabaseTokenVerifier {
    policy: DatabaseTokenPolicy,
    keys: BTreeMap<String, VerifyingKey>,
}

impl DatabaseTokenVerifier {
    pub fn from_jwks(jwks: &str, policy: DatabaseTokenPolicy) -> Result<Self, JwtValidationError> {
        if jwks.is_empty() || jwks.len() > MAX_JWKS_BYTES {
            return Err(JwtValidationError::InvalidJwks);
        }

        let document: JwksDocument =
            serde_json::from_str(jwks).map_err(|_| JwtValidationError::InvalidJwks)?;
        if document.keys.is_empty() || document.keys.len() > MAX_JWKS_KEYS {
            return Err(JwtValidationError::InvalidJwks);
        }

        let mut keys = BTreeMap::new();
        for entry in document.keys {
            validate_jwk_entry(&entry)?;
            let kid = entry.kid.clone();
            let value = serde_json::to_value(entry).map_err(|_| JwtValidationError::InvalidJwks)?;
            let key =
                VerifyingKey::from_value(value).map_err(|_| JwtValidationError::InvalidJwks)?;
            if keys.insert(kid, key).is_some() {
                return Err(JwtValidationError::DuplicateKeyId);
            }
        }

        Ok(Self { policy, keys })
    }

    pub fn verify(
        &self,
        compact_token: &str,
        requested_role: &str,
        now: i64,
    ) -> Result<VerifiedDatabaseToken, JwtValidationError> {
        if compact_token.is_empty() || compact_token.len() > MAX_TOKEN_BYTES {
            return Err(JwtValidationError::InvalidToken);
        }

        type ParsedToken = Token<DatabaseTokenClaims, Unverified<BTreeMap<String, Value>>, Compact>;
        let token: ParsedToken = compact_token
            .parse()
            .map_err(|_| JwtValidationError::InvalidToken)?;

        let kid = validate_token_header(&token)?;
        let key = self
            .keys
            .get(&kid)
            .ok_or(JwtValidationError::UnknownKeyId)?;
        let verified = token
            .verify::<_, Signature>(key)
            .map_err(|error| match error {
                TokenVerifyingError::Algorithm(_, _) => JwtValidationError::InvalidHeader,
                TokenVerifyingError::Verify(_) => JwtValidationError::InvalidSignature,
                TokenVerifyingError::Serialization(_) => JwtValidationError::InvalidToken,
            })?;
        let claims = verified
            .payload()
            .cloned()
            .ok_or(JwtValidationError::InvalidClaims)?;
        let identity = validate_claims(&claims, &self.policy, requested_role, now)?;
        let authn_id = identity.encode_authn_id()?;

        Ok(VerifiedDatabaseToken {
            claims,
            identity,
            authn_id,
        })
    }
}

pub fn decode_authn_id(value: &str) -> Result<AuthenticatedIdentity, JwtValidationError> {
    if value.is_empty() || value.len() > MAX_AUTHN_ID_BYTES {
        return Err(JwtValidationError::InvalidIdentity);
    }

    let parts = value.split(';').collect::<Vec<_>>();
    if parts.len() != 7 || parts[0] != AUTHN_ID_PREFIX {
        return Err(JwtValidationError::InvalidIdentity);
    }

    let user_id = required_part(parts[1], "u=")?.to_owned();
    let actor_value = required_part(parts[2], "actor=")?;
    let (actor_kind, actor_id) = actor_value
        .split_once(':')
        .ok_or(JwtValidationError::InvalidIdentity)?;
    let actor = match actor_kind {
        "client" => AuthenticatedActor::OAuthClient(actor_id.to_owned()),
        "credential" => AuthenticatedActor::ApiKeyCredential(actor_id.to_owned()),
        _ => return Err(JwtValidationError::InvalidIdentity),
    };
    let delegation_id = required_part(parts[3], "d=")?.to_owned();
    let auth_method = required_part(parts[4], "m=")?.parse()?;
    let authority_version_raw = required_part(parts[5], "a=")?;
    let authority_version = authority_version_raw
        .parse::<u64>()
        .map_err(|_| JwtValidationError::InvalidIdentity)?;
    if authority_version.to_string() != authority_version_raw {
        return Err(JwtValidationError::InvalidIdentity);
    }
    let profile = required_part(parts[6], "p=")?.parse()?;

    let identity = AuthenticatedIdentity {
        user_id,
        actor,
        delegation_id,
        auth_method,
        authority_version,
        profile,
    };
    validate_identity(&identity)?;
    Ok(identity)
}

pub fn decode_system_user(value: &str) -> Result<AuthenticatedIdentity, JwtValidationError> {
    let authn_id = value
        .strip_prefix(SYSTEM_USER_PREFIX)
        .ok_or(JwtValidationError::InvalidIdentity)?;
    decode_authn_id(authn_id)
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct JwksDocument {
    keys: Vec<JwkEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct JwkEntry {
    kty: String,
    crv: String,
    alg: String,
    #[serde(rename = "use")]
    usage: String,
    key_ops: Vec<String>,
    kid: String,
    x: String,
    y: String,
}

fn is_strict_https_resource(value: &str) -> bool {
    let Ok(url) = Url::parse(value) else {
        return false;
    };
    url.scheme() == "https"
        && url.host_str().is_some()
        && url.username().is_empty()
        && url.password().is_none()
        && url.query().is_none()
        && url.fragment().is_none()
}

fn validate_jwk_entry(entry: &JwkEntry) -> Result<(), JwtValidationError> {
    if entry.kty != "EC"
        || entry.crv != "P-256"
        || entry.alg != "ES256"
        || entry.usage != "sig"
        || entry.key_ops.as_slice() != ["verify"]
        || !is_valid_key_id(&entry.kid)
        || !is_canonical_p256_coordinate(&entry.x)
        || !is_canonical_p256_coordinate(&entry.y)
    {
        return Err(JwtValidationError::InvalidJwks);
    }
    Ok(())
}

fn validate_token_header(
    token: &Token<DatabaseTokenClaims, Unverified<BTreeMap<String, Value>>, Compact>,
) -> Result<String, JwtValidationError> {
    let header = token.header();
    if *header.algorithm() != AlgorithmIdentifier::ES256
        || !header.custom().is_empty()
        || header.jwk_set_url().is_some()
        || header.r#type().is_some_and(|value| value != "JWT")
        || header.certificate_url().is_some()
        || header.certificate_chain().is_some()
        || header.content_type().is_some()
        || header.critical().is_some()
        || header.key().is_some()
        || header.thumbprint().is_some()
        || header.thumbprint_sha256().is_some()
    {
        return Err(JwtValidationError::InvalidHeader);
    }

    let kid = header.key_id().ok_or(JwtValidationError::InvalidHeader)?;
    if !is_valid_key_id(kid) {
        return Err(JwtValidationError::InvalidHeader);
    }
    Ok(kid.to_owned())
}

fn validate_claims(
    claims: &DatabaseTokenClaims,
    policy: &DatabaseTokenPolicy,
    requested_role: &str,
    now: i64,
) -> Result<AuthenticatedIdentity, JwtValidationError> {
    let ttl = claims
        .expires_at
        .checked_sub(claims.issued_at)
        .ok_or(JwtValidationError::InvalidClaims)?;
    if claims.issuer != policy.issuer
        || claims.audience != policy.audience
        || claims.scope != DATABASE_SCOPE
        || claims.issued_at > now
        || claims.expires_at <= now
        || !(MIN_TOKEN_TTL_SECONDS..=MAX_TOKEN_TTL_SECONDS).contains(&ttl)
        || claims.authority_version == 0
        || claims.db_role != claims.db_profile.database_role()
        || !is_valid_internal_id(&claims.token_id)
    {
        return Err(JwtValidationError::InvalidClaims);
    }

    if requested_role != claims.db_role {
        return Err(JwtValidationError::RequestedRoleMismatch);
    }

    let actor = match (
        claims.auth_method,
        claims.client_id.as_deref(),
        claims.credential_id.as_deref(),
    ) {
        (AuthMethod::OAuth, Some(client_id), None) => {
            AuthenticatedActor::OAuthClient(client_id.to_owned())
        }
        (AuthMethod::ApiKey, None, Some(credential_id)) => {
            AuthenticatedActor::ApiKeyCredential(credential_id.to_owned())
        }
        _ => return Err(JwtValidationError::InvalidClaims),
    };

    let identity = AuthenticatedIdentity {
        user_id: claims.subject.clone(),
        actor,
        delegation_id: claims.delegation_id.clone(),
        auth_method: claims.auth_method,
        authority_version: claims.authority_version,
        profile: claims.db_profile,
    };
    validate_identity(&identity)?;
    Ok(identity)
}

fn validate_identity(identity: &AuthenticatedIdentity) -> Result<(), JwtValidationError> {
    let actor_id = match (&identity.auth_method, &identity.actor) {
        (AuthMethod::OAuth, AuthenticatedActor::OAuthClient(id)) => id,
        (AuthMethod::ApiKey, AuthenticatedActor::ApiKeyCredential(id)) => id,
        _ => return Err(JwtValidationError::InvalidIdentity),
    };
    if identity.authority_version == 0
        || !is_valid_internal_id(&identity.user_id)
        || !is_valid_internal_id(actor_id)
        || !is_valid_internal_id(&identity.delegation_id)
    {
        return Err(JwtValidationError::InvalidIdentity);
    }
    Ok(())
}

fn is_valid_internal_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_INTERNAL_ID_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

fn is_valid_key_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_KEY_ID_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
}

fn is_canonical_p256_coordinate(value: &str) -> bool {
    value.len() == 43
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

fn required_part<'a>(value: &'a str, prefix: &str) -> Result<&'a str, JwtValidationError> {
    let value = value
        .strip_prefix(prefix)
        .ok_or(JwtValidationError::InvalidIdentity)?;
    if value.is_empty() {
        return Err(JwtValidationError::InvalidIdentity);
    }
    Ok(value)
}
