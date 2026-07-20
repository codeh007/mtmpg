use std::collections::HashMap;
use std::fmt;
use std::num::NonZeroUsize;
use std::sync::Mutex;

use zeroize::Zeroizing;

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
    InvalidToken,
    Unavailable,
}

pub struct ConnectionTokenRegistry {
    capacity: usize,
    tokens: Mutex<HashMap<ConnectionId, Zeroizing<String>>>,
}

impl ConnectionTokenRegistry {
    pub fn with_capacity(capacity: usize) -> Result<Self, TokenRegistryError> {
        if capacity == 0 {
            return Err(TokenRegistryError::InvalidCapacity);
        }
        Ok(Self {
            capacity,
            tokens: Mutex::new(HashMap::with_capacity(capacity)),
        })
    }

    pub fn register(
        &self,
        connection: ConnectionId,
        token: String,
    ) -> Result<(), TokenRegistryError> {
        self.register_secret(connection, Zeroizing::new(token))
    }

    pub(crate) fn register_secret(
        &self,
        connection: ConnectionId,
        token: Zeroizing<String>,
    ) -> Result<(), TokenRegistryError> {
        if token.is_empty() || token.as_bytes().contains(&0) {
            return Err(TokenRegistryError::InvalidToken);
        }
        let mut tokens = self
            .tokens
            .lock()
            .map_err(|_| TokenRegistryError::Unavailable)?;
        if tokens.contains_key(&connection) {
            return Err(TokenRegistryError::DuplicateConnection);
        }
        if tokens.len() >= self.capacity {
            return Err(TokenRegistryError::CapacityExceeded);
        }
        tokens.insert(connection, token);
        Ok(())
    }

    pub fn claim(&self, connection: ConnectionId) -> Result<ClaimedToken, TokenRegistryError> {
        let mut tokens = self
            .tokens
            .lock()
            .map_err(|_| TokenRegistryError::Unavailable)?;
        tokens
            .remove(&connection)
            .map(ClaimedToken)
            .ok_or(TokenRegistryError::UnknownConnection)
    }

    pub fn cleanup(&self, connection: ConnectionId) -> bool {
        self.tokens
            .lock()
            .is_ok_and(|mut tokens| tokens.remove(&connection).is_some())
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.tokens.lock().map_or(usize::MAX, |tokens| tokens.len())
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
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
pub struct ClaimedToken(Zeroizing<String>);

impl ClaimedToken {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn into_secret(self) -> Zeroizing<String> {
        self.0
    }
}

impl fmt::Debug for ClaimedToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ClaimedToken([REDACTED])")
    }
}
