use crate::prelude::*;

#[derive(derive_more::Debug, derive_more::Deref)]
#[debug("Gastimator(stateless)")]
pub struct Gastimator {
    #[deref]
    dependencies: Arc<dyn GastimatorDependencies + Send + Sync>,
    state: AppState,
}

// ========================================
// Public Implementation
// ========================================
impl Gastimator {
    /// Creates a new `Gastimator` with the given dependencies.
    pub fn with(dependencies: Arc<dyn GastimatorDependencies + Send + Sync>) -> Self {
        Self {
            dependencies,
            state: AppState::default(),
        }
    }

    /// Creates a new `Gastimator` with the given local and remote gas estimators.
    pub fn with_dependencies(
        local_gas_estimator: Arc<dyn LocalTxSimulator + Send + Sync>,
        remote_gas_estimator: Arc<dyn RemoteGasEstimator + Send + Sync>,
    ) -> Self {
        Self::with(Arc::new(
            DependenciesBuilder::default()
                .local_gas_estimator(local_gas_estimator)
                .remote_gas_estimator(remote_gas_estimator)
                .build()
                .unwrap(),
        ))
    }

    /// Creates a new `Gastimator` with the given Alchemy API key.
    pub fn new(alchemy_api_key: String) -> Self {
        let remote_gas_estimator = Arc::new(AlchemyRpcClient::new(alchemy_api_key));
        let local_gas_estimator = Arc::new(RevmTxSimulator::new());
        Self::with_dependencies(local_gas_estimator, remote_gas_estimator)
    }

    /// Estimates the gas usage of `tx` using the local and remote gas estimators.
    ///
    pub async fn estimate_gas(&self, tx: Transaction) -> Result<GasEstimateResponse> {
        let start = Instant::now();
        info!("Received transaction: {:?}", tx);
        if let Some(response) = self.check_native_transfer(&tx, start)? {
            return Ok(response);
        }
        if let Some(cached) = self.use_cached_value_if_able(&tx, start)? {
            return Ok(cached);
        }
        let (local, remote) = self.compute_estimates(&tx).await?;
        self.build_response(tx, local, remote, start)
    }
}

// ========================================
// Private Implementation
// ========================================
impl Gastimator {
    /// Tries to use a cached value for the transaction if able, that is, if
    /// the transaction is considered "cacheable", and if there is a cached
    /// value for it.
    fn use_cached_value_if_able(
        &self,
        tx: &Transaction,
        start: Instant,
    ) -> Result<Option<GasEstimateResponse>> {
        if !tx.is_cacheable() {
            return Ok(None);
        }
        if let Some(cached) = self.state.cache.get(tx) {
            debug!("Found cached estimate: {:?}", cached.value());
            return Ok(Some(Self::build_response_raw(cached.clone(), start)));
        }
        Ok(None)
    }

    /// if the transaction is a native token transfer, check if the gas limit is
    /// sufficient. If it is, return the exact gas limit.
    /// If it is not, return a `GasExceedsLimit` error.
    fn check_native_transfer(
        &self,
        tx: &Transaction,
        start: Instant,
    ) -> Result<Option<GasEstimateResponse>> {
        let kind = tx.kind();
        if !kind.is_native_token_transfer() {
            return Ok(None);
        }
        let exact = Gas::exact_native_token_transfer();
        let gas_limit_or_max = tx.gas_limit_else_max();
        if gas_limit_or_max >= exact {
            Ok(Some(Self::build_response_raw(
                GasUsage::Exact { kind, gas: exact },
                start,
            )))
        } else {
            Err(Error::GasExceedsLimit {
                estimated_cost: Some(exact),
                gas_limit: gas_limit_or_max,
            })
        }
    }

    /// In parallel fetch local and remote gas estimates.
    async fn compute_estimates(&self, tx: &Transaction) -> Result<(Result<Gas>, Result<Gas>)> {
        // Allows for **parallel execution** of local and remote estimations,
        // which is possible since they are independent.
        let local = tokio::spawn({
            let estimator = self.local_gas_estimator();
            let tx = tx.clone();
            async move { estimator.locally_simulate_tx(&tx) }
        });
        let remote = tokio::spawn({
            let estimator = self.remote_gas_estimator();
            let tx = tx.clone();
            async move { estimator.estimate_gas(&tx).await }
        });
        Ok((
            local.await.map_err(Error::local_simulation_failed)?,
            remote.await.map_err(Error::remote_gas_estimate_failed)?,
        ))
    }

    fn build_response(
        &self,
        tx: Transaction,
        local: Result<Gas>,
        remote: Result<Gas>,
        start: Instant,
    ) -> Result<GasEstimateResponse> {
        let gas_limit_or_max = tx.gas_limit_else_max();
        let dont_exceed_limit = |gas: Gas| min(gas, gas_limit_or_max);
        let kind = tx.kind();

        match (local, remote) {
            (
                Err(Error::GasExceedsLimit {
                    estimated_cost,
                    gas_limit,
                }),
                Err(_), // we primarily trust the local error, which also provides the `estimated_cost`
            ) => Err(Error::GasExceedsLimit {
                estimated_cost,
                gas_limit,
            }),
            (Err(local_err), Err(remote_err)) => {
                error!("Local err: {:?}, Remote err: {:?}", local_err, remote_err);
                Err(Error::FailedToCalculateGasEstimate)
            }
            (Err(_), Ok(remote)) => {
                warn!("Local failed, using remote: {}", remote);
                Ok(Self::build_response_raw(
                    GasUsage::Estimate {
                        kind,
                        gas: dont_exceed_limit(remote),
                    },
                    start,
                ))
            }
            (Ok(local), Err(_)) => {
                warn!("Remote failed, using local: {}", local);
                Ok(Self::build_response_raw(
                    GasUsage::Estimate {
                        kind,
                        gas: dont_exceed_limit(local),
                    },
                    start,
                ))
            }
            (Ok(local), Ok(remote)) => {
                info!("Local: {}, Remote: {}", local, remote);
                // low is `min`
                let low = min(local, remote);
                // high is `max`
                // `dont_exceed_limit` is used to ensure that the gas limit is not exceeded
                let high = dont_exceed_limit(max(local, remote));
                Ok(Self::build_response_raw(
                    GasUsage::EstimateWithRange { kind, low, high },
                    start,
                ))
            }
        }
        .map(|resp| {
            if tx.is_cacheable() {
                self.state.cache.insert(tx, resp.gas_usage().clone());
            }
            resp
        })
    }

    fn build_response_raw(gas_usage: GasUsage, start: Instant) -> GasEstimateResponse {
        GasEstimateResponseBuilder::default()
            .gas_usage(gas_usage)
            .time_elapsed_in_millis(start.elapsed().as_millis())
            .build()
            .unwrap()
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
            Err(Error::LocalSimulationFailed("Hardcoded failure".to_owned()))
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
            Err(Error::RemoteGasEstimateFailed(
                "Hardcoded failure".to_owned(),
            ))
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
        let res = sut.estimate_gas(Transaction::default()).await;

        assert!(res.is_err());
    }

    #[tokio::test]
    async fn remote_fail_local_ok() {
        let local_estimate = Gas::from(60000);
        let sut = Arc::new(Sut::with_dependencies(
            LocalTxSimulatorHardCoded::new(local_estimate),
            FailRemote::new(),
        ));
        let res = sut
            .estimate_gas(Transaction::sample_contract_creation())
            .await;

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
        let res = sut
            .estimate_gas(Transaction::sample_contract_creation())
            .await;

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
        let res = sut
            .estimate_gas(Transaction::sample_native_token_transfer_gas_limit(limit))
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
        let res = sut
            .estimate_gas(Transaction::sample_contract_creation())
            .await;

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
        let res = sut
            .estimate_gas(Transaction::sample_contract_creation_gas_limit(limit))
            .await;

        let expected = &GasUsage::EstimateWithRange {
            kind: TransactionKind::ContractCreation,
            low: local_estimate,
            high: limit, // not remote_estimate
        };
        assert_eq!(res.unwrap().gas_usage(), expected);
    }
}
