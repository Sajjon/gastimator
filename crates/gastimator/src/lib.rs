mod alchemy_rpc;
mod app_state;
mod decode_rlp;
mod gastimator;
mod local_gas_estimator;
mod models;
mod remote_gas_estimator;
mod traits;

pub mod prelude {
    // INTERNAL MODULES
    pub use crate::alchemy_rpc::*;
    pub use crate::app_state::*;
    pub(crate) use crate::decode_rlp::*;
    pub use crate::gastimator::*;
    pub(crate) use crate::local_gas_estimator::*;
    pub use crate::models::*;
    pub(crate) use crate::remote_gas_estimator::*;
    pub use crate::traits::*;

    // STD
    pub use std::{
        borrow::Cow,
        cmp::{max, min},
        collections::HashMap,
        net::SocketAddr,
        sync::{Arc, RwLock},
        time::Instant,
    };

    // EXTERNAL CRATES
    pub use alloy_consensus::TxEip1559;
    pub use alloy_primitives::TxKind;
    pub use alloy_primitives::{Address, Bytes, U256};
    pub use derive_builder::Builder;
    pub use derive_more::{Deref, DerefMut};
    pub use getset::Setters;
    pub use getset::{CopyGetters, Getters};
    pub use log::{debug, error, info, warn};
    pub use reqwest::Client;
    pub use serde::{Deserialize, Serialize};
    pub use thiserror::Error as ThisError;
}

pub use prelude::*;
