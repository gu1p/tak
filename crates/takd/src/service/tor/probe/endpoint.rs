use anyhow::Result;

pub(super) fn endpoint_socket_addr(endpoint: &str) -> Result<String> {
    tak_core::endpoint::endpoint_socket_addr(endpoint).map_err(Into::into)
}

pub(super) fn endpoint_host_port(endpoint: &str) -> Result<(String, u16)> {
    tak_core::endpoint::endpoint_host_port(endpoint).map_err(Into::into)
}
