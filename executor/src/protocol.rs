use pggomtm::database_auth::{AuthMethod, DatabaseProfile};
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DelegatedPrincipal {
    pub user_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credential_id: Option<String>,
    pub delegation_id: String,
    pub auth_method: AuthMethod,
    pub authority_version: u64,
    pub database_scope: String,
    pub profile: DatabaseProfile,
    pub credential_expires_at: i64,
}

impl DelegatedPrincipal {
    #[must_use]
    pub fn actor_id(&self) -> &str {
        self.client_id
            .as_deref()
            .or(self.credential_id.as_deref())
            .unwrap_or("")
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case", deny_unknown_fields)]
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

pub fn parse_execute_request(_body: &[u8]) -> Result<ExecuteRequest, ProtocolError> {
    todo!("strict request parsing is implemented after RED verification")
}
