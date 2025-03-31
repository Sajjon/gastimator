use derive_more::IsVariant;

use crate::prelude::*;

/// Different classifications of transactions, based on the fields of
/// [`Transaction`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, IsVariant, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionKind {
    /// Only transfer of native token (ETH)
    ///
    /// Fixed gas usage of 21_000 gas
    NativeTokenTransfer,

    /// Creation of a contract e.g. ERC20, ERC721, etc.
    ///
    /// Dynamic gas usage, with **minimum usage of 32_000 gas**
    /// (EIP-2, changed this from 21_000 gas)
    ///
    /// Further gas usage depends on the contract code size
    /// 200 gas per byte of code. And EIP-170 introduced a
    /// cap of 24_576 gas for the contract creation.
    ///
    /// If the contract has a constructor, the gas usage
    /// will be higher, depending on the cost of executing
    /// the logic in the constructor.
    ///
    /// For example for external calls, loops or complex
    /// calculations the gas usage will be higher.
    ContractCreation,

    /// Call to a contract, with or without ETH transfer.
    ///
    /// Dynamic gas usage, with **minimum usage of 21_000 gas**.
    ///
    /// The gas usage depends on the contract code size
    /// and the complexity of the function being called.
    ContractCall { with_native_token_transfer: bool },

    /// Unknown transaction type, not a pure ETH transfer,
    /// might an internal transfer, possibly a self destruct?
    Unknown,
}
