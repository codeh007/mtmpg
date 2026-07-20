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
    Err(ClientAbiError::Unavailable)
}
