use std::sync::{Arc, Barrier};
use std::thread;

use mtmpg_executor::token_registry::{
    ConnectionId, ConnectionTokenRegistry, TokenRegistryError,
};

#[test]
fn token_is_claimed_exactly_once_for_its_connection() {
    let registry = ConnectionTokenRegistry::with_capacity(4).expect("bounded registry");
    let connection = ConnectionId::new(1).expect("non-null connection identity");
    registry
        .register(connection, "token-alpha".into())
        .expect("register token");

    let claimed = registry.claim(connection).expect("claim registered token");
    assert_eq!(claimed.as_str(), "token-alpha");
    assert_eq!(
        registry.claim(connection),
        Err(TokenRegistryError::UnknownConnection)
    );
}

#[test]
fn duplicate_registration_fails_without_replacing_the_original_token() {
    let registry = ConnectionTokenRegistry::with_capacity(4).expect("bounded registry");
    let connection = ConnectionId::new(2).expect("non-null connection identity");
    registry
        .register(connection, "token-original".into())
        .expect("register original token");
    assert_eq!(
        registry.register(connection, "token-replacement".into()),
        Err(TokenRegistryError::DuplicateConnection)
    );
    assert_eq!(
        registry.claim(connection).expect("claim original").as_str(),
        "token-original"
    );
}

#[test]
fn connection_failure_cleanup_removes_unclaimed_material() {
    let registry = ConnectionTokenRegistry::with_capacity(4).expect("bounded registry");
    let connection = ConnectionId::new(3).expect("non-null connection identity");
    registry
        .register(connection, "token-cleanup".into())
        .expect("register token");
    assert!(registry.cleanup(connection));
    assert!(!registry.cleanup(connection));
    assert_eq!(
        registry.claim(connection),
        Err(TokenRegistryError::UnknownConnection)
    );
}

#[test]
fn bounded_registry_fails_closed_when_full() {
    let registry = ConnectionTokenRegistry::with_capacity(1).expect("bounded registry");
    let first = ConnectionId::new(4).expect("connection identity");
    let second = ConnectionId::new(5).expect("connection identity");
    registry
        .register(first, "token-first".into())
        .expect("register first token");
    assert_eq!(
        registry.register(second, "token-second".into()),
        Err(TokenRegistryError::CapacityExceeded)
    );
    assert_eq!(
        registry.claim(first).expect("claim first token").as_str(),
        "token-first"
    );
    assert_eq!(
        registry.claim(second),
        Err(TokenRegistryError::UnknownConnection)
    );
}

#[test]
fn concurrent_connections_never_observe_another_principals_token() {
    const CONNECTIONS: usize = 32;
    let registry = Arc::new(
        ConnectionTokenRegistry::with_capacity(CONNECTIONS).expect("bounded registry"),
    );
    let barrier = Arc::new(Barrier::new(CONNECTIONS));
    let mut threads = Vec::new();

    for index in 1..=CONNECTIONS {
        let registry = Arc::clone(&registry);
        let barrier = Arc::clone(&barrier);
        threads.push(thread::spawn(move || {
            let connection = ConnectionId::new(index).expect("connection identity");
            let expected = format!("token-{index:02}");
            registry
                .register(connection, expected.clone())
                .expect("register isolated token");
            barrier.wait();
            let claimed = registry.claim(connection).expect("claim isolated token");
            assert_eq!(claimed.as_str(), expected);
        }));
    }

    for thread in threads {
        thread.join().expect("registry worker did not panic");
    }
    assert_eq!(registry.len(), 0);
}

#[test]
fn debug_output_never_contains_token_material() {
    let registry = ConnectionTokenRegistry::with_capacity(2).expect("bounded registry");
    let connection = ConnectionId::new(6).expect("connection identity");
    registry
        .register(connection, "token-sensitive".into())
        .expect("register token");
    assert!(!format!("{registry:?}").contains("token-sensitive"));
    let claimed = registry.claim(connection).expect("claim token");
    assert!(!format!("{claimed:?}").contains("token-sensitive"));
}
