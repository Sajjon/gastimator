use crate::prelude::*;

/// Dependencies of the gastimator
pub struct Dependencies {
    /// Local gas estimator
    /// (e.g. revm)
    local_gas_estimator: Arc<dyn LocalTxSimulator + Send + Sync>,

    /// Remote gas estimator
    /// (e.g. Alchemy API)
    remote_gas_estimator: Arc<dyn RemoteGasEstimator + Send + Sync>,
}

impl GastimatorDependencies for Dependencies {
    fn local_gas_estimator(&self) -> Arc<dyn LocalTxSimulator + Send + Sync> {
        self.local_gas_estimator.clone()
    }
    fn remote_gas_estimator(&self) -> Arc<dyn RemoteGasEstimator + Send + Sync> {
        self.remote_gas_estimator.clone()
    }
}

/// Trait for the gastimator dependencies, allows testing
pub trait GastimatorDependencies {
    fn local_gas_estimator(&self) -> Arc<dyn LocalTxSimulator + Send + Sync>;
    fn remote_gas_estimator(&self) -> Arc<dyn RemoteGasEstimator + Send + Sync>;
}

#[derive(derive_more::Debug, derive_more::Deref)]
#[debug("Gastimator(stateless)")]
pub struct Gastimator {
    #[deref]
    dependencies: Arc<dyn GastimatorDependencies + Send + Sync>,
    state: AppState,
}

// PUBLIC
impl Gastimator {
    pub fn with(dependencies: Arc<dyn GastimatorDependencies + Send + Sync>) -> Self {
        Self {
            dependencies,
            state: AppState::default(),
        }
    }

    pub fn with_dependencies(
        local_gas_estimator: Arc<dyn LocalTxSimulator + Send + Sync>,
        remote_gas_estimator: Arc<dyn RemoteGasEstimator + Send + Sync>,
    ) -> Self {
        Self::with(Arc::new(Dependencies {
            local_gas_estimator,
            remote_gas_estimator,
        }))
    }

    pub fn new(alchemy_api_key: String) -> Self {
        let remote_gas_estimator = Arc::new(AlchemyRpcClient::new(alchemy_api_key));
        let local_gas_estimator = Arc::new(RevmTxSimulator::new());
        Self::with_dependencies(local_gas_estimator, remote_gas_estimator)
    }

    /// Returns an estimate of the gas used by a transaction.
    pub async fn estimate_gas_canonical(
        self_: Arc<Self>,
        tx: Transaction,
    ) -> Result<GasEstimateResponse> {
        let start = Instant::now();
        info!("Received transaction: {:?}", tx);
        let gas_limit = tx.gas_limit().unwrap_or(Gas::MAX);
        let dont_exceed_limit = |gas: Gas| min(gas, gas_limit);
        let is_cacheable = tx.is_cacheable();
        let response_maybe_from_cache = |gas_usage: GasUsage, from_cache: bool| {
            if is_cacheable && !from_cache {
                // only cache transactions which are considered "cacheable" and dont
                // overwrite existing cache entries
                self_.state.cache.insert(tx.clone(), gas_usage.clone());
            }

            let time_elapsed_in_millis = start.elapsed().as_millis();

            let response = GasEstimateResponseBuilder::default()
                .gas_usage(gas_usage)
                .time_elapsed_in_millis(time_elapsed_in_millis)
                .build()
                .unwrap();

            Ok(response)
        };

        // Will cache if cacheable
        let response = |gas_usage: GasUsage| response_maybe_from_cache(gas_usage.clone(), false);

        let kind = tx.kind();
        if kind.is_native_token_transfer() {
            let exact = Gas::exact_native_token_transfer();
            if gas_limit >= exact {
                // Only if gas limit is not lower than exact can we respond directly with Ok
                return response(GasUsage::Exact { kind, gas: exact });
            } else {
                return Err(Error::GasExceedsLimit {
                    estimated_cost: Some(exact),
                    gas_limit,
                });
            }
        }

        if is_cacheable {
            // We should only try to read from the cache if we have a nonce and from address
            // otherwise we are not certain that the transaction was
            // cacheable. If the nonce is the same as the last nonce from
            // the sender, we can assume that the transaction is cacheable.
            if let Some(cached) = self_.state.cache.get(&tx) {
                debug!("Found cached estimate: {:?}", cached.value());
                return response_maybe_from_cache(cached.clone(), true);
            }
        }

        debug!("Trying local estimate...");
        let start_local = Instant::now();
        let local_estimate = self_.local_gas_estimator().locally_simulate_tx(&tx);
        debug!(
            "Local estimate took: {:?}",
            start_local.elapsed().as_millis()
        );

        debug!("Trying remote estimate...");
        let start_remote = Instant::now();
        let remote_estimate = self_.remote_gas_estimator().estimate_gas(&tx).await;
        debug!(
            "Remote estimate took: {:?}",
            start_remote.elapsed().as_millis()
        );

        match (local_estimate, remote_estimate) {
            (
                Err(Error::GasExceedsLimit {
                    estimated_cost: local_estimated_cost,
                    gas_limit: local_gas_limit,
                }),
                Err(Error::GasExceedsLimit {
                    estimated_cost: _,
                    gas_limit: _,
                }),
            ) => Err(Error::GasExceedsLimit {
                estimated_cost: local_estimated_cost,
                gas_limit: local_gas_limit,
            }),
            (Err(local_err), Err(remote_err)) => {
                error!(
                    "Failed to perform local tx simulation AND failed to fetch remote gas estimate, local err: `{:?}`, remote err: `{:?}`",
                    local_err, remote_err
                );
                Err(Error::sink("Failed to get gas estimate"))
            }
            (Err(_), Ok(remote)) => {
                warn!(
                    "Failed to perform local tx simulation, but successfully fetched remote gas estimate: `{remote}`"
                );
                let gas_usage = GasUsage::Estimate {
                    kind,
                    gas: dont_exceed_limit(remote),
                };
                response(gas_usage)
            }
            (Ok(local), Err(_)) => {
                warn!(
                    "Failed to fetch remote gas estimate, but successfully performed local tx simulation: `{local}`"
                );
                let gas_usage = GasUsage::Estimate {
                    kind,
                    gas: dont_exceed_limit(local),
                };
                response(gas_usage)
            }
            (Ok(local), Ok(remote)) => {
                info!(
                    "Successfully performed local tx simulation: `{local}`, and successfully fetched remote: `{remote}`"
                );
                let low = min(local, remote);
                let high_unbound_by_limit = max(local, remote);
                let high = dont_exceed_limit(high_unbound_by_limit);
                let gas_usage = GasUsage::EstimateWithRange { kind, low, high };
                response(gas_usage)
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    type Sut = Gastimator;

    struct FailLocal;
    impl FailLocal {
        fn new() -> Arc<Self> {
            Arc::new(Self)
        }
    }

    impl LocalTxSimulator for FailLocal {
        fn locally_simulate_tx(&self, _: &Transaction) -> Result<Gas> {
            Err(Error::sink("Failed to simulate tx"))
        }
    }

    struct LocalTxSimulatorHardCoded(Gas);
    impl LocalTxSimulator for LocalTxSimulatorHardCoded {
        fn locally_simulate_tx(&self, _: &Transaction) -> Result<Gas> {
            Ok(self.0)
        }
    }
    impl LocalTxSimulatorHardCoded {
        fn new(hardcoded: Gas) -> Arc<Self> {
            Arc::new(Self(hardcoded))
        }
    }

    struct FailRemote;
    impl FailRemote {
        fn new() -> Arc<Self> {
            Arc::new(Self)
        }
    }

    #[async_trait::async_trait]
    impl RemoteGasEstimator for FailRemote {
        async fn estimate_gas(&self, _: &Transaction) -> Result<Gas> {
            Err(Error::sink("Failed to fetch remote gas estimate"))
        }
    }

    struct RemoteHardcoded(Gas);
    #[async_trait::async_trait]
    impl RemoteGasEstimator for RemoteHardcoded {
        async fn estimate_gas(&self, _: &Transaction) -> Result<Gas> {
            Ok(self.0)
        }
    }
    impl RemoteHardcoded {
        fn new(hardcoded: Gas) -> Arc<Self> {
            Arc::new(Self(hardcoded))
        }
    }

    #[tokio::test]
    async fn fail() {
        let sut = Arc::new(Sut::with_dependencies(FailLocal::new(), FailRemote::new()));
        let res = Sut::estimate_gas_canonical(sut, Transaction::default()).await;

        assert!(res.is_err());
    }

    #[tokio::test]
    async fn remote_fail_local_ok() {
        let local_estimate = Gas::from(60000);
        let sut = Arc::new(Sut::with_dependencies(
            LocalTxSimulatorHardCoded::new(local_estimate),
            FailRemote::new(),
        ));
        let res = Sut::estimate_gas_canonical(sut, Transaction::sample_contract_creation()).await;

        let expected = &GasUsage::Estimate {
            kind: TransactionKind::ContractCreation,
            gas: local_estimate,
        };
        assert_eq!(res.unwrap().gas_usage(), expected);
    }

    #[tokio::test]
    async fn remote_ok_local_fail() {
        let remote_estimate = Gas::from(60000);
        let sut = Arc::new(Sut::with_dependencies(
            FailLocal::new(),
            RemoteHardcoded::new(remote_estimate),
        ));
        let res = Sut::estimate_gas_canonical(sut, Transaction::sample_contract_creation()).await;

        let expected = &GasUsage::Estimate {
            kind: TransactionKind::ContractCreation,
            gas: remote_estimate,
        };
        assert_eq!(res.unwrap().gas_usage(), expected);
    }

    #[tokio::test]
    async fn remote_ok_local_fail_with_limit() {
        let limit = Gas::from(10000);
        let remote_estimate = Gas::from(60000);
        let sut = Arc::new(Sut::with_dependencies(
            FailLocal::new(),
            RemoteHardcoded::new(remote_estimate),
        ));
        let res = Sut::estimate_gas_canonical(
            sut,
            Transaction::sample_native_token_transfer_gas_limit(limit),
        )
        .await;

        assert_eq!(
            res,
            Err(Error::GasExceedsLimit {
                estimated_cost: Some(Gas::exact_native_token_transfer()), // not `remote_estimate`
                gas_limit: limit,
            })
        );
    }

    #[tokio::test]
    async fn both_ok_gives_range() {
        let local_estimate = Gas::from(40000);
        let remote_estimate = Gas::from(60000);
        let sut = Arc::new(Sut::with_dependencies(
            LocalTxSimulatorHardCoded::new(local_estimate),
            RemoteHardcoded::new(remote_estimate),
        ));
        let res = Sut::estimate_gas_canonical(sut, Transaction::sample_contract_creation()).await;

        let expected = &GasUsage::EstimateWithRange {
            kind: TransactionKind::ContractCreation,
            low: local_estimate,
            high: remote_estimate,
        };
        assert_eq!(res.unwrap().gas_usage(), expected);
    }

    #[tokio::test]
    async fn both_ok_gives_range_with_limit() {
        let local_estimate = Gas::from(40000);
        let remote_estimate = Gas::from(60000);
        let limit = Gas::from(50000);
        let sut = Arc::new(Sut::with_dependencies(
            LocalTxSimulatorHardCoded::new(local_estimate),
            RemoteHardcoded::new(remote_estimate),
        ));
        let res = Sut::estimate_gas_canonical(
            sut,
            Transaction::sample_contract_creation_gas_limit(limit),
        )
        .await;

        let expected = &GasUsage::EstimateWithRange {
            kind: TransactionKind::ContractCreation,
            low: local_estimate,
            high: limit, // not remote_estimate
        };
        assert_eq!(res.unwrap().gas_usage(), expected);
    }
}
