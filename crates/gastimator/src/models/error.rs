use axum::response::IntoResponse;
use derive_more::IsVariant;

use crate::prelude::*;

impl Error {
    pub fn sink<E: std::fmt::Debug>(e: E) -> Self {
        Self::Sink {
            underlying: format!("{:?}", e).to_string(),
        }
    }
}

/// All errors which can occur during the estimation
/// of gas cost for an Ethereum transaction.
#[derive(Debug, ThisError, IsVariant, PartialEq)]
pub enum Error {
    #[error("Unknown error")]
    Unknown,
    #[error("Sink error: {underlying}")]
    Sink { underlying: String },

    #[error("Gas exceeds limit")]
    GasExceedsLimit {
        estimated_cost: Option<Gas>,
        gas_limit: Gas,
    },

    #[error("String not hex: {bad_value}")]
    StringNotHex { bad_value: String },

    #[error("Failed to parse CLI arguments: {underlying}")]
    FailedParseCliArgs { underlying: String },

    #[error("Unable to acquire cache lock")]
    UnableToAcquireCacheLock,

    #[error("Unable to start server: {underlying}")]
    UnableToStartServer { underlying: String },

    #[error("Unable to bind to address: {0}")]
    UnableToBind(String),

    #[error("Failed to get bound address: {0}")]
    UnableToGetBoundAddress(String),

    #[error("Failed to signal readiness")]
    FailedToSignalReadiness,

    #[error(
        "No Alchemy API Key provided, unable to start server. Set the `ALCHEMY_API_KEY` environment variable, e.g. `export ALCHEMY_API_KEY=your_key`, or export it in an `.envrc.secrets` file (`.envrc` already tries to load it with `direnv`)."
    )]
    NoAlchemyApiKey,

    #[error("Failed to make alchemy request")]
    AlchemySendRequest,

    #[error("Failed to parse alchemy response to type `{kind}`, underlying error: `{underlying}`")]
    AlchemyParseToResponseToType { kind: String, underlying: String },

    #[error("Failed to alchemy string response as u32")]
    AlchemyParseAsU32,

    #[error("Failed to alchemy string response as Bytes")]
    AlchemyParseAsBytes,
}

// ========================================
// Public Implementation
// ========================================
impl Error {
    pub fn start(e: std::io::Error) -> Self {
        Error::UnableToStartServer {
            underlying: e.to_string(),
        }
    }

    pub fn bind(e: std::io::Error) -> Self {
        Error::UnableToBind(e.to_string())
    }

    pub fn get_bound_address(e: std::io::Error) -> Self {
        Error::UnableToGetBoundAddress(e.to_string())
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        axum::response::Response::builder()
            .status(500)
            .body(format!("{:?}", self).into())
            .unwrap()
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
