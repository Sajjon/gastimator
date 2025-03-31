use alloy_consensus::TxEip1559;
use alloy_primitives::TxKind;
use getset::Setters;

use crate::prelude::*;

#[derive(
    Default, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Builder, Getters, Setters,
)]
#[builder(setter(into), default)]
pub struct Transaction {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[getset(get = "pub")]
    nonce: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[getset(get = "pub")]
    from: Option<Address>,

    /// The 160-bit address of the message call’s recipient or, for a contract creation
    /// transaction, ∅, used here to denote the only member of B0 ; formally Tt.
    #[getset(get = "pub")]
    to: TxKind,

    /// A scalar value equal to the number of Wei to
    /// be transferred to the message call’s recipient or,
    /// in the case of contract creation, as an endowment
    /// to the newly created account; formally Tv.
    #[serde(default)]
    #[getset(get = "pub")]
    value: U256,

    /// Transaction is not allowed to cost more than this limit.
    #[getset(get = "pub", set = "pub")]
    gas_limit: Option<Gas>,

    /// Input has two uses depending if transaction is Create or Call (if `to` field is None or
    /// Some). pub init: An unlimited size byte array specifying the
    /// EVM-code for the account initialisation procedure CREATE,
    /// data: An unlimited size byte array specifying the
    /// input data of the message call, formally Td.
    #[serde(alias = "data")]
    #[serde(default)]
    #[getset(get = "pub")]
    input: Bytes,
}

impl Transaction {
    /// We should only try to read from the cache if we have a nonce and from address
    /// otherwise we are not certain that the transaction was
    /// cacheable. If the nonce is the same as the last nonce from
    /// the sender, we can assume that the transaction is cacheable.
    pub fn is_cacheable(&self) -> bool {
        self.nonce().is_some() && self.from().is_some()
    }

    pub fn gas_usage_classification(&self, estimate: impl Into<Option<Gas>>) -> GasUsage {
        let kind = self.kind();
        match kind {
            TransactionKind::NativeTokenTransfer => GasUsage::Exact {
                kind,
                exact: Gas::exact_native_token_transfer(),
            },
            TransactionKind::ContractCreation => GasUsage::AtLeast {
                kind,
                at_least: Gas::min_contract_creation(),
            },
            TransactionKind::ContractCall {
                with_native_token_transfer,
            } => {
                let at_least = Gas::min_contract_call(with_native_token_transfer);
                GasUsage::at_least_with_estimate(kind, at_least, estimate)
            }
            TransactionKind::Unknown => {
                let at_least = Gas::exact_native_token_transfer();
                GasUsage::at_least_with_estimate(kind, at_least, estimate)
            }
        }
    }

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
}

impl TryFrom<RawTransaction> for Transaction {
    type Error = crate::Error;

    fn try_from(value: RawTransaction) -> Result<Self> {
        let tx = decode_eip1559_transaction(value.rlp.as_ref())?;
        Ok(tx.into())
    }
}

impl Transaction {
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
impl From<TxEip1559> for Transaction {
    fn from(value: TxEip1559) -> Self {
        Self::from_eip1559(value)
    }
}
