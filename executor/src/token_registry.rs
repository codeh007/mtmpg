use std::fmt;
use std::num::NonZeroUsize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConnectionId(NonZeroUsize);

impl ConnectionId {
    #[must_use]
    pub const fn new(value: usize) -> Option<Self> {
        match NonZeroUsize::new(value) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenRegistryError {
    InvalidCapacity,
    DuplicateConnection,
    UnknownConnection,
    CapacityExceeded,
}

pub struct ConnectionTokenRegistry {
    _capacity: usize,
}

impl ConnectionTokenRegistry {
    pub fn with_capacity(capacity: usize) -> Result<Self, TokenRegistryError> {
        if capacity == 0 {
            return Err(TokenRegistryError::InvalidCapacity);
        }
        Ok(Self {
            _capacity: capacity,
        })
    }

    pub fn register(
        &self,
        _connection: ConnectionId,
        _token: String,
    ) -> Result<(), TokenRegistryError> {
        todo!("per-connection token registration is implemented after RED verification")
    }

    pub fn claim(&self, _connection: ConnectionId) -> Result<ClaimedToken, TokenRegistryError> {
        todo!("one-time token claiming is implemented after RED verification")
    }

    pub fn cleanup(&self, _connection: ConnectionId) -> bool {
        todo!("failure cleanup is implemented after RED verification")
    }

    #[must_use]
    pub fn len(&self) -> usize {
        todo!("registry accounting is implemented after RED verification")
    }
}

impl fmt::Debug for ConnectionTokenRegistry {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ConnectionTokenRegistry")
            .finish_non_exhaustive()
    }
}

#[derive(PartialEq, Eq)]
pub struct ClaimedToken(String);

impl ClaimedToken {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for ClaimedToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ClaimedToken([REDACTED])")
    }
}
