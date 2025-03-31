use crate::prelude::*;

use derive_more::IsVariant;

/// Gas usage classification for a transaction.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, IsVariant, Serialize, Deserialize, derive_more::Display,
)]
#[serde(rename_all = "snake_case")]
pub enum GasUsage {
    /// We know exactly the gas usage of the transaction.
    #[display("exact({gas})")]
    Exact {
        /// Kind of transaction, identified by this software based
        /// on the fields of a `Transaction` value
        kind: TransactionKind,
        /// The **exact** amount of gas used by the transaction.
        gas: Gas,
    },

    /// We do not know the exact gas usage of the transaction, but an
    /// estimate. **It is NOT guaranteed that the actual gas usage is equal
    /// to this estimate**, it can be higher or lower.
    #[display("estimate({gas})")]
    Estimate {
        /// Kind of transaction, identified by this software based
        /// on the fields of a `Transaction` value
        kind: TransactionKind,
        /// Estimated gas usage
        gas: Gas,
    },

    /// We do not know the exact gas usage of the transaction, but we
    /// have estimated a value within a range. **It is NOT guaranteed
    /// that the actual gas usage is inside this range**.
    #[display("estimate_with_range({low} - {high})")]
    EstimateWithRange {
        /// Kind of transaction, identified by this software based
        /// on the fields of a `Transaction` value
        kind: TransactionKind,
        /// Low bound estimate
        low: Gas,
        /// High bound estimate
        high: Gas,
    },
}

// ========================================
// Public Implementation
// ========================================
impl GasUsage {
    /// Returns the transaction kind
    pub fn transaction_kind(&self) -> &TransactionKind {
        match self {
            Self::Estimate { kind, .. } => kind,
            Self::EstimateWithRange { kind, .. } => kind,
            Self::Exact { kind, .. } => kind,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type Sut = GasUsage;

    #[test]
    fn json_snapshot_exact() {
        insta::assert_json_snapshot!(&Sut::Exact {
            kind: TransactionKind::NativeTokenTransfer,
            gas: Gas::exact_native_token_transfer(),
        })
    }

    #[test]
    fn json_snapshot_estimate() {
        insta::assert_json_snapshot!(&Sut::Estimate {
            kind: TransactionKind::ContractCreation,
            gas: Gas::from(54321),
        })
    }

    #[test]
    fn json_snapshot_estimate_with_range() {
        insta::assert_json_snapshot!(&Sut::EstimateWithRange {
            kind: TransactionKind::ContractCreation,
            low: Gas::from(43210),
            high: Gas::from(54321),
        })
    }
}
