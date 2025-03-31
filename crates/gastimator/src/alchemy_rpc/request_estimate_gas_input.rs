use crate::prelude::*;

/// The input for the `eth_estimateGas` method, to be used with the
/// [`AlchemyRpcClient`].
///
/// For more info [see Alchemy's documentation][doc]
///
/// [doc]: https://docs.alchemy.com/reference/eth-estimategas
#[derive(Clone, Debug, Serialize, Builder, Getters, Default)]
#[builder(setter(into), default)]
pub struct AlchemyEstimateGasInput {
    /// The address of the recipient of the transaction, either a contract or an EOA.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[getset(get = "pub")]
    to: Option<Address>, // e.g. "0xd46e8dd67c5d32be8058bb8eb970870f07244567",

    /// An optional gas limit
    #[serde(skip_serializing_if = "Option::is_none")]
    #[getset(get = "pub")]
    gas: Option<U256>, // e.g. "0x0",

    /// An optional gas price
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "gasPrice")]
    #[getset(get = "pub")]
    gas_price: Option<U256>, // e.g. "0x9184e72a000",

    /// An optional amount of ETH to send with the transaction
    /// (in wei).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[getset(get = "pub")]
    value: Option<U256>, // e.g. "0x0",

    /// An optional data field, which can be used to send data with the transaction.
    /// This is typically used for contract calls.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[getset(get = "pub")]
    data: Option<Bytes>, // e.g. "0x"
}

impl From<Transaction> for AlchemyEstimateGasInput {
    /// Converts a `Transaction` into an `AlchemyEstimateGasInput`.
    fn from(value: Transaction) -> Self {
        let data = if value.input().is_empty() {
            None
        } else {
            Some(value.input().clone())
        };
        let gas_limit = value.gas_limit().map(|gas| U256::from(*gas));

        AlchemyEstimateGasInputBuilder::default()
            .to(*value.to())
            .gas(gas_limit)
            .value(*value.value())
            .data(data)
            .build()
            .unwrap()
    }
}

// ========================================
// IsRpcRequest impl
// ========================================
impl IsRpcRequest for AlchemyEstimateGasInput {
    type Param = AlchemyEstimateGasInput;
    fn method() -> String {
        "eth_estimateGas".to_owned()
    }
}
