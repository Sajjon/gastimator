use crate::prelude::*;

/// Run configuration for the server
#[derive(Debug, Clone, Builder, Getters, CopyGetters)]
#[builder(setter(into))]
pub struct ServerConfig {
    /// The address our program is running on.
    /// E.g. "0.0.0.0"
    #[getset(get = "pub")]
    address: String,

    /// The port our program is running on.
    /// E.g. `3000`
    #[getset(get_copy = "pub")]
    port: u16,
}

// ========================================
// Public Implementation
// ========================================
impl ServerConfig {
    /// Returns the server address and port as a string
    pub fn address_with_port(&self) -> String {
        format!("{}:{}", self.address, self.port)
    }
}
