use crate::prelude::*;

#[derive(Debug, Default)]
pub struct AppState {
    pub cache: Cache,
}

#[derive(Clone, Debug, Default, Deref, DerefMut)]
pub struct Cache(dashmap::DashMap<Transaction, GasUsage>);

#[derive(Debug, Clone, Serialize, Deserialize, Builder, Getters)] // deserialize for tests
#[builder(setter(into))]
pub struct GasEstimateResponse {
    /// The gas used by the transaction.
    #[getset(get = "pub")]
    gas_usage: GasUsage,

    #[getset(get = "pub")]
    time_elapsed_in_millis: u128,
}
