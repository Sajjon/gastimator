use crate::prelude::*;

pub struct Dependencies {
    local_gas_estimator: Arc<dyn LocalTxSimulator + Send + Sync>,
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
        let response_maybe_from_cache = |estimate: GasUsage, from_cache: bool| {
            if is_cacheable && !from_cache {
                self_.state.cache.insert(tx.clone(), estimate.clone());
            }
            let time_elapsed_in_millis = start.elapsed().as_millis();
            let response = GasEstimateResponseBuilder::default()
                .gas_usage(estimate)
                .time_elapsed_in_millis(time_elapsed_in_millis)
                .build()
                .unwrap();
            Ok(response)
        };

        // Will cache if cacheable
        let response = |estimate: GasUsage| response_maybe_from_cache(estimate.clone(), false);

        let usage_classification = tx.gas_usage_classification(None);
        if let GasUsage::Exact { exact, .. } = &usage_classification {
            if gas_limit >= *exact {
                // Only if gas limit is not lower than exact can we respond directly with Ok
                return response(usage_classification);
            } else {
                return Err(Error::GasExceedsLimit {
                    estimated_cost: Some(*exact),
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
                response(usage_classification.with_estimate(dont_exceed_limit(remote)))
            }
            (Ok(local), Err(_)) => {
                warn!(
                    "Failed to fetch remote gas estimate, but successfully performed local tx simulation: `{local}`"
                );
                response(usage_classification.with_estimate(dont_exceed_limit(local)))
            }
            (Ok(local), Ok(remote)) => {
                info!(
                    "Successfully performed local tx simulation: `{local}`, and successfully fetched remote: `{remote}`"
                );
                let estimate = std::cmp::max(local, remote);
                response(usage_classification.with_estimate(dont_exceed_limit(estimate)))
            }
        }
    }
}
