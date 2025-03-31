#![allow(non_snake_case)]

use gastimator_rest::prelude::*;
use log::trace;

fn alchemy_api_key() -> String {
    read_alchemy_api_key().unwrap_display()
}

/// A tester for the gastimate server, allowing us to send
/// API requests and assert the responses.
///
/// The requests uses `reqwest::Client`.
///
/// Use only the `test` function, which will handle the setup and cleanup.
///
/// The async closure passed to `test` will not be called until the server
/// is ready to receive requests.
struct Tester {
    server_handle: tokio::task::JoinHandle<()>,
    client: Client,
    url: String,
}
impl Tester {
    async fn test<Fut>(test: impl Fn(Arc<Self>) -> Fut)
    where
        Fut: std::future::Future<Output = ()>,
    {
        let tester = Arc::new(Tester::_new().await);
        test(tester.clone()).await;
        Arc::try_unwrap(tester)
            .ok()
            .expect("Should be able to consume Arc and get Tester") // No Debug needed
            ._cleanup();
    }

    async fn _new() -> Self {
        // Arrange: Spawn the server
        let server_config = ServerConfigBuilder::default()
            .port(0u16)
            .address("0.0.0.0")
            .build()
            .unwrap();
        let config = ConfigBuilder::default()
            .server(server_config)
            .alchemy_api_key(alchemy_api_key())
            .build()
            .unwrap();
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
        let server_handle = tokio::spawn(async move {
            run_signaling_readiness(&config, ready_tx)
                .await
                .expect("Server failed to start");
        });
        // Wait for the server to signal readiness and get the bound address
        let bound_address = ready_rx.await.expect("Failed to receive server address");
        let url = format!("http://127.0.0.1:{}", bound_address.port());

        let client = Client::new();

        Self {
            server_handle,
            client,
            url,
        }
    }

    async fn estimate(
        &self,
        input: &Transaction,
    ) -> std::result::Result<GasEstimateResponse, String> {
        let response = self
            .client
            .post(format!("{}/tx", self.url))
            .json(&input)
            .send()
            .await
            .map_err(|e| format!("{:?}", e))?;

        let status = response.status();

        let body_bytes = response.bytes().await.map_err(|e| format!("{:?}", e))?;

        let body_string = String::from_utf8_lossy(&body_bytes);

        trace!(
            "🔮 RAW response: Status = {}, Body = {:?}",
            status, body_string
        );

        let model = serde_json::from_slice::<GasEstimateResponse>(&body_bytes)
            .map_err(|_| body_string.into_owned())?;

        Ok(model)
    }

    fn _cleanup(self) {
        // Cleanup: Abort the server task
        self.server_handle.abort();
    }
}

#[tokio::test]
async fn native_token_transfer() {
    // ARRANGE
    let input = &Transaction::sample_native_token_transfer();
    Tester::test(|tester| async move {
        // ACT
        let response = tester.estimate(input).await.unwrap();

        // ASSERT
        pretty_assertions::assert_eq!(
            *response.gas_usage(),
            GasUsage::Exact {
                kind: TransactionKind::NativeTokenTransfer,
                gas: 21_000.into()
            }
        );
    })
    .await;
}

#[tokio::test]
async fn call_data_limit_too_low() {
    // ARRANGE
    // https://etherscan.io/tx/0x4a6307830781fa2a308d315a0daaa5770419fab1104cf6d650167c470e7f9b5a
    let rlp = hex_literal::hex!(
        "02f902db01820168841dcd6500843d831e6783027a6d9466a9893cc07d91d95644aedd05d03f95e1dba8af8803bbae1324948000b9026424856bc30000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000008000000000000000000000000000000000000000000000000000000000000000020b080000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000a00000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000003bbae1324948000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000096cdea52111684fd74ec6cdf31dd97f395737a5d00000000000000000000000000000000000000000000000003bbae132494800000000000000000000000000000000000000000000000001e28ba62f4e8c66e7b00000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000002000000000000000000000000c02aaa39b223fe8d0a0e5c4f27ead9083c756cc20000000000000000000000000aff507ac29b8cea2fb10d2ad14408c2d79a35adc001a0ff907e592e412943d4f7136aeb55f9fcf701ef295c7cd620be07ffe037de5b58a05889a72e62156c986ccc657bcdba1f91c481e817ee607408320d312416fa3a67"
    );
    let input = RawTransaction {
        rlp: Bytes::from(rlp),
    };
    let mut input = Transaction::try_from(input).unwrap();
    // change limit to something much too low
    let gas_limit = Gas::from(123);
    input.set_gas_limit(Some(gas_limit));
    let input = &input;
    Tester::test(|tester| async move {
        // ACT
        let result = tester.estimate(input).await;
        // ASSERT
        assert!(result.err().unwrap().contains(&format!(
            "GasExceedsLimit {{ estimated_cost: Some(Gas(24648)), gas_limit: Gas({}) }}",
            gas_limit
        )));
    })
    .await;
}

#[tokio::test]
async fn transfer_limit_too_low() {
    // ARRANGE
    let gas_limit = Gas::from(1);
    let input = &Transaction::sample_native_token_transfer_gas_limit(gas_limit);
    Tester::test(|tester| async move {
        // ACT
        let result = tester.estimate(input).await;
        // ASSERT
        assert!(result.err().unwrap().contains(&format!(
            "GasExceedsLimit {{ estimated_cost: Some(Gas(21000)), gas_limit: Gas({}) }}",
            gas_limit
        )));
    })
    .await;
}

#[tokio::test]
async fn GIVEN__cached_response__WHEN__cache_hit__THEN__response_time_is_fast() {
    // ARRANGE
    Tester::test(|tester| async move {
        // ACT
        let input = &Transaction::sample_native_token_transfer_cachable();
        let first = tester.estimate(input).await;
        let second = tester.estimate(input).await;

        let time_first = first.as_ref().unwrap().time_elapsed_in_millis();
        let time_second = second.as_ref().unwrap().time_elapsed_in_millis();

        // ASSERT
        // actually Rust is so fast that both times are 0
        assert!(time_second <= time_first);
    })
    .await;
}
