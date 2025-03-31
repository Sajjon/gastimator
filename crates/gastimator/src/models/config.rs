use crate::prelude::*;

/// Run configuration for the server
/// requires an `alchemy_api_key` and a [`Route`] config.
#[derive(Debug, Clone, Builder, Getters)]
#[builder(setter(into))]
pub struct Config {
    #[getset(get = "pub")]
    server: ServerConfig,

    #[getset(get = "pub")]
    alchemy_api_key: String,
}

impl Config {
    /// Returns the server address and port as a string
    pub fn address_with_port(&self) -> String {
        self.server.address_with_port()
    }
}

/// Tries to read the Alchemy API key from the environment variable `ALCHEMY_API_KEY`,
/// as a String.
///
/// # Throws
/// Throws [`Error::NoAlchemyApiKey`] if the environment variable is not set.
pub fn read_alchemy_api_key() -> Result<String> {
    std::env::var("ALCHEMY_API_KEY").map_err(|_| Error::NoAlchemyApiKey)
}
