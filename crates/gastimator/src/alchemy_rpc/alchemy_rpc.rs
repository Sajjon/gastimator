use crate::prelude::*;

const ALCHEMY_ETHEREUM_BASE_URL: &str = "https://eth-mainnet.g.alchemy.com/v2";
const ALCHEMY_GAS_USE_EXCEEDS_LIMIT_ERROR: &str = "gas required exceeds allowance";

pub struct AlchemyRpcClient {
    api_key: String,
    client: reqwest::Client,
    id_stepper: IdStepper,
}

impl AlchemyRpcClient {
    pub fn new(api_key: impl AsRef<str>) -> Self {
        Self {
            api_key: api_key.as_ref().to_owned(),
            client: reqwest::Client::default(),
            id_stepper: IdStepper::default(),
        }
    }
}

impl AlchemyRpcClient {
    fn url(&self) -> String {
        format!("{}/{}", ALCHEMY_ETHEREUM_BASE_URL, self.api_key)
    }

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
        let request = RpcRequestBuilder::default()
            .params(params.into_iter().collect::<Vec<Req::Param>>())
            .method(Req::method())
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
            .map_err(|_| Error::AlchemySendRequest)?;

        let status = response.status();
        info!("Alchemy response status: {:?}", status);
        let body_bytes = response.bytes().await.map_err(Error::sink)?;
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
}

impl AlchemyRpcClient {
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

trait TryIntoU64 {
    fn try_into_u64(self) -> Result<u64>;
}
impl TryIntoU64 for U256 {
    fn try_into_u64(self) -> Result<u64> {
        if self > U256::from(u64::MAX) {
            return Err(Error::sink("U256 is too big, does not fit in 64 bits"));
        }
        Ok(self.as_limbs()[0])
    }
}

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
