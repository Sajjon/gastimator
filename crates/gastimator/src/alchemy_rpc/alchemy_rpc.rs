use crate::prelude::*;

/// The url of the Alchemy Ethereum API
const ALCHEMY_ETHEREUM_BASE_URL: &str = "https://eth-mainnet.g.alchemy.com/v2";

/// The error message returned by Alchemy when the gas limit is exceeded
/// when estimating gas for a transaction. Unfortunately, the Alchemy API
/// does not return a proper error code, so we have to rely on the error message
/// to determine if the gas limit was exceeded. A bit hacky, but it works.
const ALCHEMY_GAS_USE_EXCEEDS_LIMIT_ERROR: &str = "gas required exceeds allowance";

/// Alchemy RPC client for estimating gas costs
///
/// It implements the `RemoteGasEstimator` trait, which allows it to be used
/// as a gas estimator in the `GasEstimator` struct.
pub struct AlchemyRpcClient {
    /// The API key for the Alchemy Ethereum API, typically read
    /// from the environment variable `ALCHEMY_API_KEY`.
    api_key: String,

    /// An underlying HTTP client for making requests to the Alchemy API,
    /// using the reqwest library.
    /// This client is used to send JSON-RPC requests to the Alchemy API.
    client: reqwest::Client,

    /// A helper which generates unique request IDs for each JSON-RPC request.
    id_stepper: IdStepper,
}

// ========================================
// Public Implementation
// ========================================

impl AlchemyRpcClient {
    /// Creates a new Alchemy RPC client with the given API key.
    ///
    /// # Parameters
    /// - `api_key`: The API key for the Alchemy Ethereum API. You can
    /// pass either a `String` or a string slice (`&str`).
    ///
    /// # Returns
    /// A new instance of `AlchemyRpcClient`.
    pub fn new(api_key: impl AsRef<str>) -> Self {
        Self {
            api_key: api_key.as_ref().to_owned(),
            client: reqwest::Client::default(),
            id_stepper: IdStepper::default(),
        }
    }
}

// ========================================
// Private Implementation
// ========================================

impl AlchemyRpcClient {
    /// Formats the URL for the Alchemy Ethereum API using the provided API key.
    fn url(&self) -> String {
        format!("{}/{}", ALCHEMY_ETHEREUM_BASE_URL, self.api_key)
    }

    /// Calls the RPC method of the `Req::method()` using a single parameter
    /// value. You can intercept the body of the response and pre-process it
    /// before deserializing it into the `Res` type.
    ///
    /// # Parameters
    /// - `param`: The parameter value to be passed to the RPC method.
    /// - `utf8_body_interceptor`: A closure that takes the body of the response
    ///   as a `Cow<str>` and returns an `Option<Result<Res>>`. If you return e.g.
    /// `Some(Err(...))`, the function will return that error. If you don't want to
    /// intercept the body, you can pass `|_| None` as the interceptor.
    ///
    /// # Returns
    /// A `Result<Res>` containing the deserialized response or an error.
    async fn call_single<Req, Res>(
        &self,
        param: Req,
        utf8_body_interceptor: impl FnOnce(Cow<'_, str>) -> Option<Result<Res>>,
    ) -> Result<Res>
    where
        Req: IsRpcRequest,
        Req::Param: Clone,
        Res: for<'de> Deserialize<'de>,
        Req: TyEq<Req::Param>, // #20041
    {
        self.call::<Req, Res>([param.cast()], utf8_body_interceptor)
            .await
    }

    /// Calls the RPC method of the `Req::method()` using with multiple parameters.
    /// You can intercept the body of the response and pre-process it
    /// before deserializing it into the `Res` type.
    ///
    /// # Parameters
    /// - `params`: The parameter values to be passed to the RPC method.
    /// - `utf8_body_interceptor`: A closure that takes the body of the response
    ///   as a `Cow<str>` and returns an `Option<Result<Res>>`. If you return e.g.
    /// `Some(Err(...))`, the function will return that error. If you don't want to
    /// intercept the body, you can pass `|_| None` as the interceptor.
    ///
    /// # Returns
    /// A `Result<Res>` containing the deserialized response or an error.
    async fn call<Req, Res>(
        &self,
        params: impl IntoIterator<Item = Req::Param>,
        utf8_body_interceptor: impl FnOnce(Cow<'_, str>) -> Option<Result<Res>>,
    ) -> Result<Res>
    where
        Req: IsRpcRequest,
        Req::Param: Clone,
        Res: for<'de> Deserialize<'de>,
    {
        let id = self.id_stepper.next();
        let method = Req::method();
        let request = RpcRequestBuilder::default()
            .params(params.into_iter().collect::<Vec<Req::Param>>())
            .method(method.clone())
            .id(id)
            .build()
            .unwrap();

        #[cfg(debug_assertions)]
        {
            let json = serde_json::to_string_pretty(&request).unwrap();
            debug!("👻 Alchemy request JSON: {:?}", json);
        }

        let response = self
            .client
            .post(self.url())
            .json(&request)
            .send()
            .await
            .map_err(|_| Error::AlchemySendRequest { method })?;

        let status = response.status();
        info!("Alchemy response status: {:?}", status);
        let body_bytes = response
            .bytes()
            .await
            .map_err(Error::alchemy_read_bytes_of_response)?;
        let body_string = String::from_utf8_lossy(&body_bytes);

        // Print the response body as a debug string
        #[cfg(debug_assertions)]
        debug!(
            "🔮 Alchemy RAW response: Status = {}, Body = {:?}",
            status, body_string
        );

        if let Some(intercepted) = utf8_body_interceptor(body_string) {
            return intercepted;
        }

        serde_json::from_slice(&body_bytes).map_err(|e| Error::AlchemyParseToResponseToType {
            kind: std::any::type_name::<Res>().to_owned(),
            underlying: format!("{:?}", e),
        })
    }

    /// Calls the `eth_estimateGas` method of the Alchemy API to estimate the gas cost
    /// for a given transaction.
    ///
    /// # Parameters
    /// - `input`: The input parameters for the `eth_estimateGas` method. For more
    /// info see [`AlchemyEstimateGasInput`].
    ///
    /// # Returns
    /// A `Result<Gas>` containing the estimated gas cost or an error.
    async fn get_gas_estimate(&self, input: AlchemyEstimateGasInput) -> Result<Gas> {
        let gas_limit = *input.gas();

        let response: RpcResponse = self
            .call_single(input, |body| {
                if body.contains(ALCHEMY_GAS_USE_EXCEEDS_LIMIT_ERROR) {
                    let gas_limit = gas_limit
                        .expect("Should not have failed with gas required exceed limit if there is no limit")
                        .try_into_u64()
                        .expect("Gas limit should fit in a u64");
                    let gas_limit = Gas::from(gas_limit);
                    Some(Err(Error::GasExceedsLimit {
                        estimated_cost: None,
                        gas_limit,
                    }))
                } else {
                    None
                }
            })
            .await?;

        let response = &response.result_strip_0x();
        let gas_used = u64::from_str_radix(response, 16).map_err(|_| Error::AlchemyParseAsU32)?;
        info!(
            "Successfully fetched gas estimate from Alchemy: {:?}",
            gas_used
        );
        Ok(gas_used.into())
    }
}

/// A trait for converting a U256 value into a u64 value.
///
/// This is a custom trait because the `U256` type does not implement
/// `TryInto<u64>` directly. And because we do not own the `U256` type,
/// we cannot implement `TryInto<u64>` for it. So we create our own
/// trait to provide this functionality.
trait TryIntoU64 {
    /// Attempts to convert a U256 value into a u64 value.
    fn try_into_u64(self) -> Result<u64>;
}

impl TryIntoU64 for U256 {
    fn try_into_u64(self) -> Result<u64> {
        if self > U256::from(u64::MAX) {
            return Err(Error::UInt256LargerThanU64);
        }
        Ok(self.as_limbs()[0])
    }
}

// ========================================
// RemoteGasEstimator Implementation
// ========================================

#[async_trait::async_trait]
impl RemoteGasEstimator for AlchemyRpcClient {
    async fn estimate_gas(&self, tx: &Transaction) -> Result<Gas> {
        let tx = AlchemyEstimateGasInput::from(tx.clone());
        self.get_gas_estimate(tx).await
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn uint256_as_u64() {
        let u256 = U256::from(237u64);
        let u64 = u256.try_into_u64().unwrap();
        assert_eq!(u64, 237u64)
    }

    #[test]
    fn uint256_as_u64_too_large() {
        let u256 = U256::from(u128::MAX);
        let res = u256.try_into_u64();
        assert!(res.is_err())
    }
}
