use crate::prelude::*;

use derive_more::IsVariant;

/// Gas usage classification for a transaction.
#[derive(Debug, Clone, PartialEq, Eq, Hash, IsVariant, Serialize, Deserialize)]
pub enum GasUsage {
    /// We know exactly the gas usage of the transaction.
    Exact {
        kind: TransactionKind,
        /// The **exact** amount of gas used by the transaction.
        exact: Gas,
    },
    /// We know the minimum gas usage of the transaction.
    /// But no estimate for actual gas usage has yet been
    /// calculated.
    AtLeast {
        kind: TransactionKind,
        /// The **minimum** amount of gas used by the transaction.
        at_least: Gas,
    },
    /// We know the minimum gas usage of the transaction,
    /// with an estimated actual gas usage
    AtLeastWithEstimate {
        kind: TransactionKind,
        /// The **minimum** amount of gas used by the transaction.
        at_least: Gas,
        /// An estimated amount of gas used by the transaction,
        estimate: Gas,
    },
}

impl GasUsage {
    pub fn at_least_with_estimate(
        kind: TransactionKind,
        at_least: Gas,
        estimate: impl Into<Option<Gas>>,
    ) -> Self {
        if let Some(estimate) = estimate.into() {
            Self::AtLeastWithEstimate {
                kind,
                at_least,
                estimate,
            }
        } else {
            Self::AtLeast { kind, at_least }
        }
    }

    pub fn with_estimate(self, estimate: Gas) -> Self {
        match self {
            Self::Exact { .. } => panic!("Should not have fetched an estimate for Exact"),
            Self::AtLeast { kind, at_least } => Self::AtLeastWithEstimate {
                kind,
                at_least,
                estimate,
            },
            Self::AtLeastWithEstimate { .. } => panic!("Should not re-estimate"),
        }
    }

    pub fn transaction_kind(&self) -> &TransactionKind {
        match self {
            Self::AtLeast { kind, .. } => kind,
            Self::AtLeastWithEstimate { kind, .. } => kind,
            Self::Exact { kind, .. } => kind,
        }
    }
}
