#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientAbiError {
    Unavailable,
    UnsupportedPostgresMajor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientAbi {
    pub postgresql_major: u32,
    pub oauth_bearer_auth_data_hook: bool,
    pub async_cancel: bool,
    pub extended_query: bool,
    pub socket_poll: bool,
}

pub fn current_client_abi() -> Result<ClientAbi, ClientAbiError> {
    // SAFETY: PQlibVersion takes no arguments and does not retain Rust memory.
    let version = unsafe { ffi::PQlibVersion() };
    if version <= 0 {
        return Err(ClientAbiError::Unavailable);
    }
    let postgresql_major =
        u32::try_from(version / 10_000).map_err(|_| ClientAbiError::Unavailable)?;
    if postgresql_major != 18 {
        return Err(ClientAbiError::UnsupportedPostgresMajor);
    }
    Ok(ClientAbi {
        postgresql_major,
        oauth_bearer_auth_data_hook: true,
        async_cancel: true,
        extended_query: true,
        socket_poll: true,
    })
}

#[allow(
    dead_code,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals
)]
mod ffi {
    include!(concat!(
        env!("OUT_DIR"),
        "/mtmpg_executor_libpq_bindings.rs"
    ));
}
