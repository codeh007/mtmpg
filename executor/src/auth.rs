use std::collections::BTreeMap;
use std::sync::Mutex;

use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

pub const AUTH_WINDOW_SECONDS: i64 = 30;
pub const EXECUTE_PATH: &str = "/v1/sql/execute";
pub const WIRE_VERSION: &str = "v1";

type HmacSha256 = Hmac<Sha256>;

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
    secret: Zeroizing<Vec<u8>>,
    replay_capacity: usize,
    replay: Mutex<BTreeMap<String, i64>>,
}

impl HmacAuthenticator {
    pub fn new(secret: Vec<u8>, replay_capacity: usize) -> Result<Self, AuthenticationError> {
        if secret.len() != 32 || replay_capacity == 0 {
            return Err(AuthenticationError::InvalidConfiguration);
        }
        Ok(Self {
            secret: Zeroizing::new(secret),
            replay_capacity,
            replay: Mutex::new(BTreeMap::new()),
        })
    }

    pub fn verify(&self, request: &SignedRequest<'_>, now: i64) -> Result<(), AuthenticationError> {
        if request.method != "POST"
            || request.path != EXECUTE_PATH
            || request.version != WIRE_VERSION
            || now.abs_diff(request.timestamp) > AUTH_WINDOW_SECONDS.unsigned_abs()
            || !is_lower_hex(request.nonce, 32)
            || !is_lower_hex(request.signature, 64)
        {
            return Err(AuthenticationError::Unauthorized);
        }

        let signature =
            decode_signature(request.signature).ok_or(AuthenticationError::Unauthorized)?;
        let body_digest = Sha256::digest(request.body);
        let canonical = format!(
            "{}\n{}\n{}\n{}\n{}\n{body_digest:x}",
            request.version, request.method, request.path, request.timestamp, request.nonce
        );
        let mut mac = HmacSha256::new_from_slice(&self.secret)
            .map_err(|_| AuthenticationError::Unauthorized)?;
        mac.update(canonical.as_bytes());
        mac.verify_slice(&signature)
            .map_err(|_| AuthenticationError::Unauthorized)?;

        let mut replay = self
            .replay
            .lock()
            .map_err(|_| AuthenticationError::Unauthorized)?;
        replay.retain(|_, expires_at| *expires_at >= now);
        if replay.contains_key(request.nonce) || replay.len() >= self.replay_capacity {
            return Err(AuthenticationError::Unauthorized);
        }
        replay.insert(
            request.nonce.to_owned(),
            request.timestamp.saturating_add(AUTH_WINDOW_SECONDS),
        );
        Ok(())
    }
}

fn is_lower_hex(value: &str, expected_bytes: usize) -> bool {
    value.len() == expected_bytes * 2
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn decode_signature(value: &str) -> Option<[u8; 32]> {
    if !is_lower_hex(value, 32) {
        return None;
    }

    let mut decoded = [0_u8; 32];
    for (output, pair) in decoded.iter_mut().zip(value.as_bytes().chunks_exact(2)) {
        *output = (hex_nibble(pair[0])? << 4) | hex_nibble(pair[1])?;
    }
    Some(decoded)
}

fn hex_nibble(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        _ => None,
    }
}
