use pggomtm::database_auth::{AuthMethod, DatabaseProfile};
use serde::{Deserialize, Deserializer, Serialize, de};
use serde_json::Value;

pub const MAX_REQUEST_BODY_BYTES: usize = 256 * 1024;
pub const MAX_STATEMENT_BYTES: usize = 64 * 1024;
pub const MAX_BIND_COUNT: usize = 64;
pub const MAX_BIND_VALUE_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolError {
    InvalidRequest,
    LimitExceeded,
    ConfirmationRequired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DelegatedPrincipal {
    pub user_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credential_id: Option<String>,
    pub delegation_id: String,
    pub auth_method: AuthMethod,
    pub authority_version: u64,
    pub database_scope: String,
    pub profile: DatabaseProfile,
    pub credential_expires_at: Option<i64>,
}

impl<'de> Deserialize<'de> for DelegatedPrincipal {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct WireDelegatedPrincipal {
            user_id: String,
            #[serde(default)]
            client_id: Option<String>,
            #[serde(default)]
            credential_id: Option<String>,
            delegation_id: String,
            auth_method: AuthMethod,
            authority_version: u64,
            database_scope: String,
            profile: DatabaseProfile,
            #[serde(default, deserialize_with = "deserialize_credential_expiry")]
            credential_expires_at: RequiredCredentialExpiry,
        }

        let wire = WireDelegatedPrincipal::deserialize(deserializer)?;
        let credential_expires_at = match wire.credential_expires_at {
            RequiredCredentialExpiry::Missing => {
                return Err(de::Error::missing_field("credential_expires_at"));
            }
            RequiredCredentialExpiry::Present(value) => value,
        };
        Ok(Self {
            user_id: wire.user_id,
            client_id: wire.client_id,
            credential_id: wire.credential_id,
            delegation_id: wire.delegation_id,
            auth_method: wire.auth_method,
            authority_version: wire.authority_version,
            database_scope: wire.database_scope,
            profile: wire.profile,
            credential_expires_at,
        })
    }
}

#[derive(Default)]
enum RequiredCredentialExpiry {
    #[default]
    Missing,
    Present(Option<i64>),
}

fn deserialize_credential_expiry<'de, D>(
    deserializer: D,
) -> Result<RequiredCredentialExpiry, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<i64>::deserialize(deserializer).map(RequiredCredentialExpiry::Present)
}

impl DelegatedPrincipal {
    #[must_use]
    pub fn actor_id(&self) -> &str {
        self.client_id
            .as_deref()
            .or(self.credential_id.as_deref())
            .unwrap_or("")
    }

    pub(crate) fn is_valid(&self) -> bool {
        let actor_matches = matches!(
            (self.auth_method, &self.client_id, &self.credential_id),
            (AuthMethod::OAuth, Some(_), None) | (AuthMethod::ApiKey, None, Some(_))
        );
        let credential_expiry_matches = match (self.auth_method, self.credential_expires_at) {
            (AuthMethod::OAuth, Some(expires_at)) | (AuthMethod::ApiKey, Some(expires_at)) => {
                expires_at > 0
            }
            (AuthMethod::ApiKey, None) => true,
            (AuthMethod::OAuth, None) => false,
        };
        actor_matches
            && is_internal_id(&self.user_id)
            && is_internal_id(self.actor_id())
            && is_internal_id(&self.delegation_id)
            && self.authority_version > 0
            && self.database_scope == "database"
            && credential_expiry_matches
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum BindValue {
    Null,
    Text(String),
    Int64(i64),
    Boolean(bool),
    Json(Value),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionIntent {
    Read,
    Change,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecuteRequest {
    pub principal: DelegatedPrincipal,
    pub statement: String,
    pub binds: Vec<BindValue>,
    pub intent: ExecutionIntent,
    pub change_confirmed: bool,
    pub correlation_id: String,
}

pub fn parse_execute_request(body: &[u8]) -> Result<ExecuteRequest, ProtocolError> {
    if body.len() > MAX_REQUEST_BODY_BYTES {
        return Err(ProtocolError::LimitExceeded);
    }

    let request: ExecuteRequest =
        serde_json::from_slice(body).map_err(|_| ProtocolError::InvalidRequest)?;
    if !request.principal.is_valid()
        || request.statement.trim().is_empty()
        || !is_correlation_id(&request.correlation_id)
    {
        return Err(ProtocolError::InvalidRequest);
    }
    if request.statement.len() > MAX_STATEMENT_BYTES
        || request.binds.len() > MAX_BIND_COUNT
        || request.binds.iter().any(bind_exceeds_limit)
    {
        return Err(ProtocolError::LimitExceeded);
    }

    match (request.intent, request.change_confirmed) {
        (ExecutionIntent::Read, false) | (ExecutionIntent::Change, true) => Ok(request),
        (ExecutionIntent::Change, false) => Err(ProtocolError::ConfirmationRequired),
        (ExecutionIntent::Read, true) => Err(ProtocolError::InvalidRequest),
    }
}

fn bind_exceeds_limit(bind: &BindValue) -> bool {
    match bind {
        BindValue::Null | BindValue::Int64(_) | BindValue::Boolean(_) => false,
        BindValue::Text(value) => value.len() > MAX_BIND_VALUE_BYTES,
        BindValue::Json(value) => match serde_json::to_vec(value) {
            Ok(encoded) => encoded.len() > MAX_BIND_VALUE_BYTES,
            Err(_) => true,
        },
    }
}

fn is_internal_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

fn is_correlation_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
}
