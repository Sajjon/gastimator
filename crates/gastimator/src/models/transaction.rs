use crate::prelude::*;

/// Transaction is a struct that represents a transaction in the Ethereum network.
///
/// The most important fields are `to`, `value`, and `input`.
///
/// It does not contain any [EIP-1559][eip] fields (`max_priority_fee_per_gas` / `max_fee_per_gas`)
/// as those are not relevant for gas estimation, only gas cost.
///
/// [eip]: https://eips.ethereum.org/EIPS/eip-1559
#[derive(
    Default, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Builder, Getters, Setters,
)]
#[builder(setter(into), default)]
pub struct Transaction {
    /// Optional nonce, being a monotonic counter of how many transactions an account
    /// has made. Can be used together with `from` to identify
    /// if a transaction is cacheable. If two transactions are identical but lack
    /// a nonce or a from address, they are not cacheable.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[getset(get = "pub")]
    nonce: Option<u64>,

    /// Optional sender address, can be used together with `nonce` to identify
    /// if a transaction is cacheable. If two transactions are identical but lack
    /// a nonce or a from address, they are not cacheable.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[getset(get = "pub")]
    from: Option<Address>,

    /// The receiver of the transaction, either a contract or an EOA.
    #[getset(get = "pub")]
    to: TxKind,

    /// The amount of ETH to send with the transaction (in wei).
    #[serde(default)]
    #[getset(get = "pub")]
    value: U256,

    /// Transaction is not allowed to cost more than this limit.
    #[getset(get = "pub", set = "pub")]
    gas_limit: Option<Gas>,

    /// Input data used for contract calls or creation, or empty for pure ETH transfers.
    #[serde(alias = "data")]
    #[serde(default)]
    #[getset(get = "pub")]
    input: Bytes,
}

// ========================================
// Public Implementation
// ========================================
impl Transaction {
    /// We should only try to read from the cache if we have a nonce and from address
    /// otherwise we are not certain that the transaction was
    /// cacheable. If the nonce is the same as the last nonce from
    /// the sender, we can assume that the transaction is cacheable.
    pub fn is_cacheable(&self) -> bool {
        self.nonce().is_some() && self.from().is_some()
    }

    /// Classifies this transaction into a kind, either a pure ETH transfer,
    /// contract creation, contract call or unknown.
    pub fn kind(&self) -> TransactionKind {
        let to_is_none = self.to.to().is_none(); // Contract creation check
        let is_call = self.to.is_call();
        let is_create = self.to.is_create();
        let value_is_zero = self.value.is_zero();
        let data_is_empty = self.input.is_empty();

        if !to_is_none && !value_is_zero && data_is_empty {
            // Pure ETH Transfer
            TransactionKind::NativeTokenTransfer
        } else if is_create && !data_is_empty {
            // Contract Creation (must have init code)
            TransactionKind::ContractCreation
        } else if is_call {
            // Contract Call (can have ETH transfer)
            TransactionKind::ContractCall {
                with_native_token_transfer: !value_is_zero,
            }
        } else {
            // Fallback to Unknown (no more SelfDestruct assumption)
            TransactionKind::Unknown
        }
    }

    /// Creates a new transaction from an EIP-1559 (alloy) transaction.
    pub fn from_eip1559(value: TxEip1559) -> Self {
        let gas_limit = if value.gas_limit == 0 {
            None
        } else {
            Some(Gas::from(value.gas_limit))
        };

        TransactionBuilder::default()
            .nonce(value.nonce)
            .gas_limit(gas_limit)
            .to(value.to)
            .value(value.value)
            .input(value.input)
            .build()
            .unwrap()
    }
}

// ========================================
// From Implementations
// ========================================

impl TryFrom<RawTransaction> for Transaction {
    type Error = crate::Error;

    fn try_from(value: RawTransaction) -> Result<Self> {
        let tx = decode_eip1559_transaction(value.rlp.as_ref())?;
        Ok(tx.into())
    }
}

impl From<TxEip1559> for Transaction {
    fn from(value: TxEip1559) -> Self {
        Self::from_eip1559(value)
    }
}

// ========================================
// Sample Values (test helpers)
// ========================================

// this is not the correct flag, we want test only, but `test` flag is not set
// across crates, and I'm too lazy to add a specific test feature for this...
#[cfg(debug_assertions)]
impl Transaction {
    /// A sample value for a native token transfer transaction, with
    /// an optional gas limit.
    ///
    /// This is used for testing purposes only.
    pub fn sample_native_token_transfer_gas_limit(limit: impl Into<Option<Gas>>) -> Self {
        TransactionBuilder::default()
            .to(Address::from([0x12; 20]))
            .value(U256::from(1))
            .gas_limit(limit)
            .build()
            .unwrap()
    }

    /// A sample value for a native token transfer transaction.
    ///
    /// This is used for testing purposes only.
    pub fn sample_native_token_transfer() -> Self {
        Self::sample_native_token_transfer_gas_limit(None)
    }

    /// A sample value for a native token transfer transaction,
    /// which has a nonce and from address.
    ///
    /// This is used for testing purposes only.
    pub fn sample_native_token_transfer_cachable() -> Self {
        TransactionBuilder::default()
            .nonce(1)
            .from(Address::from([0x12; 20]))
            .value(U256::from(1))
            .to(Address::from([0x12; 20]))
            .build()
            .unwrap()
    }

    /// A sample value for a contract creation transaction, with
    /// an optional gas limit.
    ///
    /// This is used for testing purposes only.
    pub fn sample_contract_creation_gas_limit(limit: impl Into<Option<Gas>>) -> Self {
        TransactionBuilder::default()
            .to(TxKind::Create)
            .input(Bytes::from([0xab; 400])) // ERC20 contracts are often around 400 bytes
            .gas_limit(limit)
            .build()
            .unwrap()
    }

    /// A sample value for a contract creation transaction, with
    /// an optional gas limit.
    ///
    /// This is used for testing purposes only.
    pub fn sample_contract_creation() -> Self {
        Self::sample_contract_creation_gas_limit(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type Sut = Transaction;

    #[test]
    fn is_cachable() {
        assert!(Sut::sample_native_token_transfer_cachable().is_cacheable());
        assert!(!Sut::sample_native_token_transfer().is_cacheable());
    }
}
