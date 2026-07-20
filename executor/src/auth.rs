pub const AUTH_WINDOW_SECONDS: i64 = 30;
pub const EXECUTE_PATH: &str = "/v1/sql/execute";
pub const WIRE_VERSION: &str = "v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthenticationError {
    InvalidConfiguration,
    Unauthorized,
}

#[derive(Debug, Clone, Copy)]
pub struct SignedRequest<'a> {
    pub method: &'a str,
    pub path: &'a str,
    pub version: &'a str,
    pub timestamp: i64,
    pub nonce: &'a str,
    pub body: &'a [u8],
    pub signature: &'a str,
}

pub struct HmacAuthenticator {
    _secret: Vec<u8>,
    _replay_capacity: usize,
}

impl HmacAuthenticator {
    pub fn new(secret: Vec<u8>, replay_capacity: usize) -> Result<Self, AuthenticationError> {
        if secret.is_empty() || replay_capacity == 0 {
            return Err(AuthenticationError::InvalidConfiguration);
        }
        Ok(Self {
            _secret: secret,
            _replay_capacity: replay_capacity,
        })
    }

    pub fn verify(
        &self,
        _request: &SignedRequest<'_>,
        _now: i64,
    ) -> Result<(), AuthenticationError> {
        todo!("HMAC authentication is implemented after RED verification")
    }
}
