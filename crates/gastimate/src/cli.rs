pub use clap::Parser;
pub use gastimator::prelude::*;

#[derive(Parser, Debug)]
#[command(name = "gastimate", version)]
#[command(author = "Alexander Cyon <alex.cyon@gmail.com>")]
#[command(
    about = "Ethereum Gas Estimator",
    long_about = "Estimates the Gas cost of a transaction on the Ethereum network."
)]
pub struct Cli {
    /// The address of the server
    #[arg(short = 'a', long = "address", default_value = "0.0.0.0")]
    pub address: String,

    /// The port our program is running on.
    /// Valid values are 0-65535 (TCP standard range).
    #[arg(short = 'p', long = "port", default_value_t = 3000)]
    pub port: u16,

    #[arg(short = 'k', long = "key", default_value = None)]
    pub alchemy_api_key: Option<String>,
}

impl From<Cli> for ServerConfig {
    fn from(args: Cli) -> Self {
        ServerConfigBuilder::default()
            .address(args.address)
            .port(args.port)
            .build()
            .unwrap()
    }
}

impl TryFrom<Cli> for Config {
    type Error = Error;
    fn try_from(args: Cli) -> Result<Self> {
        let alchemy_api_key = args
            .alchemy_api_key
            .clone()
            .or_else(|| read_alchemy_api_key().ok())
            .ok_or(Error::NoAlchemyApiKey)?;
        let server_config = ServerConfig::from(args);
        Ok(ConfigBuilder::default()
            .server(server_config)
            .alchemy_api_key(alchemy_api_key)
            .build()
            .unwrap())
    }
}
