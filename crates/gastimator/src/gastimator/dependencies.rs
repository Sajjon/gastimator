use crate::prelude::*;

/// Dependencies of the gastimator
#[derive(Builder)]
#[builder(setter(into))]
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
