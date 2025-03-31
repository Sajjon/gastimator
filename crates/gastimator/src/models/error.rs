use axum::response::IntoResponse;
use derive_more::IsVariant;

use crate::prelude::*;

/// All errors which can occur during the estimation
/// of gas cost for an Ethereum transaction.
#[derive(Debug, ThisError, IsVariant, PartialEq)]
pub enum Error {
    /// Gas usage of transaction exceeds specifed gas limit
    #[error("Gas exceeds limit")]
    GasExceedsLimit {
        estimated_cost: Option<Gas>,
        gas_limit: Gas,
    },

    /// Failed to decode a String as hex.
    #[error("String not hex: {bad_value}")]
    StringNotHex { bad_value: String },

    /// Failed to parse CLI arguments from clap
    #[error("Failed to parse CLI arguments: {underlying}")]
    FailedParseCliArgs { underlying: String },

    /// Unable to acquire cache lock
    #[error("Unable to acquire cache lock")]
    UnableToAcquireCacheLock,

    /// Unable to start REST server
    #[error("Unable to start server: {underlying}")]
    UnableToStartServer { underlying: String },

    /// Unable to bind to address
    #[error("Unable to bind to address: {0}")]
    UnableToBind(String),

    /// Both remote and local estimate failed
    #[error("Failed to calculate gas")]
    FailedToCalculateGasEstimate,

    /// Local simulation failed
    #[error("Local TX simulation failed: {0}")]
    LocalSimulationFailed(String),

    /// Remote gas estimate failed
    #[error("Remote gas estimate failed: {0}")]
    RemoteGasEstimateFailed(String),

    /// Unable to get address of bound socket
    #[error("Failed to get bound address: {0}")]
    UnableToGetBoundAddress(String),

    /// Failed to signal readiness
    #[error("Failed to signal readiness")]
    FailedToSignalReadiness,

    /// No Alchemy API key provided
    #[error(
        "No Alchemy API Key provided, unable to start server. Set the `ALCHEMY_API_KEY` environment variable, e.g. `export ALCHEMY_API_KEY=your_key`, or export it in an `.envrc.secrets` file (`.envrc` already tries to load it with `direnv`)."
    )]
    NoAlchemyApiKey,

    /// Failed to send Alchemy RPC request
    #[error("Failed to make alchemy request, method: `{method}`")]
    AlchemySendRequest { method: String },

    /// Failed to read Alchemy response as Bytes
    #[error("Failed to read Alchemy response Bytes, underlying error: `{underlying}`")]
    AlchemyReadBytesOfResponse { underlying: String },

    /// Failed to parse Alchemy response to some generic type
    #[error("Failed to parse Alchemy response to type `{kind}`, underlying error: `{underlying}`")]
    AlchemyParseToResponseToType { kind: String, underlying: String },

    /// Failed to parse Alchemy String response as u32
    #[error("Failed to parse Alchemy String response as u32")]
    AlchemyParseAsU32,

    /// Failed to parse Alchemy String response as Bytes
    #[error("Failed to parse Alchemy String response as Bytes")]
    AlchemyParseAsBytes,

    /// Failed to cast a UIn256 to u64, would not fit
    #[error("UInt256 larger than u64")]
    UInt256LargerThanU64,

    /// Failed to RLP decode bytes into EIP1559 transaction
    #[error(
        "Failed to RLP decode bytes into EIP1559 transaction, underlying error: `{underlying}`"
    )]
    DecodeRlpFailedBytesIntoEip1559Tx { underlying: String },

    /// Failed to RLP decode bytes into a Signed EIP1559 transaction
    #[error(
        "Failed to RLP decode bytes into a Signed EIP1559 transaction, underlying error: `{underlying}`"
    )]
    DecodeRlpFailedBytesIntoSignedEip1559Tx { underlying: String },
}

// ========================================
// Public Implementation
// ========================================
impl Error {
    pub fn local_simulation_failed(e: impl std::fmt::Display) -> Self {
        Error::LocalSimulationFailed(e.to_string())
    }

    pub fn remote_gas_estimate_failed(e: impl std::fmt::Display) -> Self {
        Error::LocalSimulationFailed(e.to_string())
    }

    pub fn alchemy_read_bytes_of_response(e: impl std::fmt::Display) -> Self {
        Error::AlchemyReadBytesOfResponse {
            underlying: e.to_string(),
        }
    }

    pub fn decode_rlp_decode_bytes_into_eip1559(e: impl std::fmt::Display) -> Self {
        Error::DecodeRlpFailedBytesIntoEip1559Tx {
            underlying: e.to_string(),
        }
    }

    pub fn decode_rlp_decode_bytes_into_signed_eip1559(e: impl std::fmt::Display) -> Self {
        Error::DecodeRlpFailedBytesIntoSignedEip1559Tx {
            underlying: e.to_string(),
        }
    }

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
