use crate::prelude::*;

/// Route configuration for endpoint
/// returns gas estimate for a transaction
#[derive(Debug, Clone, Builder, Getters, CopyGetters)]
#[builder(setter(into))]
pub struct ServerConfig {
    /// E.g. "0.0.0.0"
    #[getset(get = "pub")]
    address: String,

    /// E.g. `3000`
    #[getset(get_copy = "pub")]
    port: u16,
}

impl ServerConfig {
    /// Returns the server address and port as a string
    pub fn address_with_port(&self) -> String {
        format!("{}:{}", self.address, self.port)
    }
}
