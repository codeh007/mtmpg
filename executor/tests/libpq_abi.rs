use mtmpg_executor::libpq::{ClientAbiError, current_client_abi};

#[test]
fn generated_client_binding_matches_the_current_postgresql_18_header() {
    let abi = current_client_abi().expect("generated PostgreSQL 18 client ABI");
    assert_eq!(abi.postgresql_major, 18);
    assert!(abi.oauth_bearer_auth_data_hook);
    assert!(abi.async_cancel);
    assert!(abi.extended_query);
    assert!(abi.socket_poll);
}

#[test]
fn unsupported_or_missing_client_abi_fails_closed() {
    assert_ne!(
        current_client_abi(),
        Err(ClientAbiError::UnsupportedPostgresMajor)
    );
}
