use crate::prelude::*;

/// Amount of gas used by a transaction.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    derive_more::Deref,
    derive_more::From,
    derive_more::Display,
)]
#[serde(transparent)]
pub struct Gas(u64);

// ========================================
// Public Implementation
// ========================================
impl Gas {
    pub const MAX: Self = Self(u64::MAX);

    /// Fixed gas usage for a native token transfer
    pub fn exact_native_token_transfer() -> Self {
        Self(21_000)
    }

    /// Minimum gas usage for a contract creation
    pub fn min_contract_creation() -> Self {
        Self(32_000)
    }

    /// EIP-150 sets the gas cost of CALL and CALLCODE to 700 gas
    /// https://eips.ethereum.org/EIPS/eip-150
    pub fn base_contract_call_cost() -> Self {
        Self(700)
    }

    /// Minimum gas usage for a contract call, depending on
    /// `with_native_token_transfer` flag.
    pub fn min_contract_call(with_native_token_transfer: bool) -> Self {
        if with_native_token_transfer {
            // Ethereum Yellow Paper, appendix G: Gas Costs, CALL opcode

            let stipend_receiving = 900;
            let non_zero_value_transfer = 1000;

            // 2600
            Self(*Self::base_contract_call_cost() + stipend_receiving + non_zero_value_transfer)
        } else {
            Self::base_contract_call_cost()
        }
    }
}
