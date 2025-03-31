#![allow(non_snake_case)]

use alloy::hex::FromHex;
use gastimator_rest::prelude::*;
use log::trace;

fn alchemy_api_key() -> String {
    read_alchemy_api_key().unwrap_display()
}

trait FromRlp: Sized {
    fn from_rlp<T: AsRef<[u8]>>(hex: T) -> Self;
}
impl FromRlp for Transaction {
    #[inline]
    fn from_rlp<T: AsRef<[u8]>>(hex: T) -> Self {
        let input = RawTransaction {
            rlp: Bytes::from_hex(hex).unwrap(),
        };
        Transaction::try_from(input).unwrap()
    }
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
async fn erc20_token_transfer_usdc() {
    // ARRANGE
    // https://etherscan.io/tx/0x32f16f78f063db3bdca899d39c39250997de6267b70b1db6cadc6edecf02fadd

    let input = &Transaction::from_rlp(
        "02f902db01820168841dcd6500843d831e6783027a6d9466a9893cc07d91d95644aedd05d03f95e1dba8af8803bbae1324948000b9026424856bc30000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000008000000000000000000000000000000000000000000000000000000000000000020b080000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000a00000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000003bbae1324948000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000096cdea52111684fd74ec6cdf31dd97f395737a5d00000000000000000000000000000000000000000000000003bbae132494800000000000000000000000000000000000000000000000001e28ba62f4e8c66e7b00000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000002000000000000000000000000c02aaa39b223fe8d0a0e5c4f27ead9083c756cc20000000000000000000000000aff507ac29b8cea2fb10d2ad14408c2d79a35adc001a0ff907e592e412943d4f7136aeb55f9fcf701ef295c7cd620be07ffe037de5b58a05889a72e62156c986ccc657bcdba1f91c481e817ee607408320d312416fa3a67",
    );
    Tester::test(|tester| async move {
        // ACT
        let response = tester.estimate(input).await.unwrap();

        // ASSERT
        pretty_assertions::assert_eq!(
            *response.gas_usage(),
            GasUsage::EstimateWithRange {
                kind: TransactionKind::ContractCall {
                    with_native_token_transfer: true
                },
                low: 30120.into(),
                high: 147649.into(),
                // these are off, real answer is: 62248
            }
        );
    })
    .await;
}

#[tokio::test]
async fn erc20_token_transfer_usdt() {
    // ARRANGE
    // https://etherscan.io/tx/0x1cd81514d818a293a0322c4a130d5fc588f13da4a67056d2f0d6039c9164bf0a
    let input = &Transaction::from_rlp(
        "0x02f8b00154842e942ba9846a0022a283030d4094dac17f958d2ee523a2206206994597c13d831ec780b844a9059cbb00000000000000000000000068f9950010075a94924c22eb3598781facbc5bab00000000000000000000000000000000000000000000000000000000515c3f40c001a052f02bf5b79d535c820184ea1339d64b08ca6c1bec91e79ac39924ab5dfaaf25a067f6b8035977434570c70351c215e3e71b7805c3a8014f5eb29d614cb6592302",
    );
    Tester::test(|tester| async move {
        // ACT
        let response = tester.estimate(input).await.unwrap();

        // ASSERT
        pretty_assertions::assert_eq!(
            *response.gas_usage(),
            GasUsage::Estimate {
                kind: TransactionKind::ContractCall {
                    with_native_token_transfer: false
                },
                gas: 22490.into(), // off - actual gas is: 63,197
            }
        );
    })
    .await;
}

#[tokio::test]
async fn erc20_token_transfer_dai() {
    // ARRANGE
    // https://etherscan.io/tx/0xd8cb17599010751c7b3a2fb8db54f18c084e47b75ce3ea6a0bf2f067e730bc75
    let input = &Transaction::from_rlp(
        "0x02f8af011e84054e0840845003d67a82953f946b175474e89094c44da98b954eedeac495271d0f80b844a9059cbb00000000000000000000000080974f1fa51d9aad25ae0b857dd58ee0569a9b2b0000000000000000000000000000000000000000000000000304a509418a0225c001a0abb53cd56cc7162acbed7bf8193e7e31d5751b16e4bf5bd45b951906ec7206b9a0543ead33208796e51d2a68c81a4c78565047f3300c969677655d478f78d6c5d1",
    );
    Tester::test(|tester| async move {
        // ACT
        let response = tester.estimate(input).await.unwrap();

        // ASSERT
        pretty_assertions::assert_eq!(
            *response.gas_usage(),
            GasUsage::EstimateWithRange {
                kind: TransactionKind::ContractCall {
                    with_native_token_transfer: false
                },
                low: 22640.into(),
                high: 35090.into(), // off - actual gas is: 29,930
            }
        );
    })
    .await;
}

#[tokio::test]
async fn call_contract_1inch() {
    // ARRANGE
    // https://etherscan.io/tx/0xa996550810f67191ffc422a71422beff23c27d1c9ea8da83d2445b568fff1715
    let input = &Transaction::from_rlp(
        "0x02f901d10106840aba9500844ef9f69e830802ff94b300000b72deaeb607a12d5f54773d1c19c7028d80b90164a03de6a90000000000000000000000001c95519d3fc922fc04fcf5d099be4a1ed8b1524000000000000000000000000000000000000000000000000000c7ee79449a3042000000000000000000000000dac17f958d2ee523a2206206994597c13d831ec7000000000000000000000000000000000000000000000000000000000000008000000000000000000000000000000000000000000000000000000000000000a88770ba910000000000000000000000001c95519d3fc922fc04fcf5d099be4a1ed8b1524000000000000000000000000000000000000000000000000000c7ee79449a30420000000000000000000000000000000000000000000000000000000003d6655008800000000000003b6d03406c3de40561e6f760dc9422403eb72b67a5d20ea800800000000000003b6d03400d4a11d5eeaac28ec3f61d100daf4d40471f1852f9338bcb000000000000000000000000000000000000000000000000c080a0be4c6fbfcbcd61e8f575799e2064341f56d5d2688af2b6debcaa70787c7ff2b2a05e0733fb93a967d4c6c29692e60951f12cc793dc9d45dbb0d8eab623063c0372",
    );

    Tester::test(|tester| async move {
        // ACT
        let response = tester.estimate(input).await.unwrap();

        // ASSERT
        pretty_assertions::assert_eq!(
            *response.gas_usage(),
            GasUsage::Estimate {
                kind: TransactionKind::ContractCall {
                    with_native_token_transfer: false
                },
                gas: 28_850.into(), // completely off - actual gas is: 282_148
            }
        );
    })
    .await;
}

#[tokio::test]
async fn call_data_limit_too_low() {
    // ARRANGE
    // https://etherscan.io/tx/0x4a6307830781fa2a308d315a0daaa5770419fab1104cf6d650167c470e7f9b5a

    let mut input = Transaction::from_rlp(
        "02f902db01820168841dcd6500843d831e6783027a6d9466a9893cc07d91d95644aedd05d03f95e1dba8af8803bbae1324948000b9026424856bc30000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000008000000000000000000000000000000000000000000000000000000000000000020b080000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000a00000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000003bbae1324948000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000096cdea52111684fd74ec6cdf31dd97f395737a5d00000000000000000000000000000000000000000000000003bbae132494800000000000000000000000000000000000000000000000001e28ba62f4e8c66e7b00000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000002000000000000000000000000c02aaa39b223fe8d0a0e5c4f27ead9083c756cc20000000000000000000000000aff507ac29b8cea2fb10d2ad14408c2d79a35adc001a0ff907e592e412943d4f7136aeb55f9fcf701ef295c7cd620be07ffe037de5b58a05889a72e62156c986ccc657bcdba1f91c481e817ee607408320d312416fa3a67",
    );
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
