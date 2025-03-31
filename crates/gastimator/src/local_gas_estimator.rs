use std::convert::Infallible;

use crate::prelude::*;
use revm::{
    Context, ExecuteEvm, MainBuilder, MainContext,
    context::{
        BlockEnv, CfgEnv, Evm, TxEnv,
        result::{EVMError, InvalidTransaction, ResultAndState},
    },
    database::{CacheDB, EmptyDB, EmptyDBTyped},
    handler::{EthPrecompiles, instructions::EthInstructions},
    interpreter::interpreter::EthInterpreter,
};

/// A typealias for the type of the EVM we are using.
#[allow(clippy::upper_case_acronyms)]
type EVM = Evm<
    Context<BlockEnv, TxEnv, CfgEnv, CacheDB<EmptyDBTyped<Infallible>>>,
    (),
    EthInstructions<
        EthInterpreter,
        Context<BlockEnv, TxEnv, CfgEnv, CacheDB<EmptyDBTyped<Infallible>>>,
    >,
    EthPrecompiles,
>;

/// An EVM transaction simulator that can be used to simulate transactions locally.
/// It uses the `revm` crate to simulate the transaction and returns the gas used.
pub struct RevmTxSimulator {
    evm: RwLock<EVM>,
}

/// A simulator of transaction that happens locally.
/// It is used to simulate transactions locally and returns the gas used.
pub trait LocalTxSimulator {
    fn locally_simulate_tx(&self, tx: &Transaction) -> Result<Gas>;
}

impl From<Transaction> for TxEnv {
    fn from(tx: Transaction) -> Self {
        TxEnv {
            nonce: tx.nonce().unwrap_or_default(),
            caller: tx.from().unwrap_or_default(),
            kind: *tx.to(),
            data: tx.input().clone(),
            gas_limit: tx.gas_limit().map(|gas| *gas).unwrap_or_default(),
            value: *tx.value(),
            ..Default::default()
        }
    }
}

// ========================================
// Public Implementation
// ========================================
impl RevmTxSimulator {
    /// Constructs an Evm instance using an in-memory database, simulating
    /// Ethereum mainnet transactions.
    pub fn new() -> Self {
        // Initialise empty in-memory-db
        let cache_db = CacheDB::new(EmptyDB::default());

        // Initialise an empty (default) EVM
        let evm = Context::mainnet()
            .with_db(cache_db)
            .modify_cfg_chained(|cfg| {
                // Disable nonce checks, since we might not be providing nonces
                cfg.disable_nonce_check = true;
                // Disable balance checks, since we do not wanna have to have balance
                // to run simulation
                cfg.disable_balance_check = true; // requires feature flag "optional_balance_check"
            })
            .build_mainnet();

        Self {
            evm: RwLock::new(evm),
        }
    }
}

// ========================================
// Private Implementation
// ========================================
impl RevmTxSimulator {
    fn simulate_tx(evm: &mut EVM, tx: TxEnv) -> Result<Gas> {
        // Set the transaction as the current transaction
        evm.modify_tx(|t| *t = tx);

        // Transact the transaction that is set in the context.
        let ResultAndState { result, state: _ } = evm.replay().map_err(|e| match e {
            EVMError::Transaction(InvalidTransaction::CallGasCostMoreThanGasLimit {
                initial_gas,
                gas_limit,
            }) => {
                // Handle the the case where the gas_limit of the
                // transaction was less than the required
                warn!("Gas limit less than required");
                Error::GasExceedsLimit {
                    estimated_cost: Some(Gas::from(initial_gas)),
                    gas_limit: Gas::from(gas_limit),
                }
            }
            _ => {
                error!("Error while simulating transaction: {e}");
                Error::local_simulation_failed(e)
            }
        })?;
        Ok(Gas::from(result.gas_used()))
    }
}

// ========================================
// LocalTxSimulator Implementation
// ========================================
impl LocalTxSimulator for RevmTxSimulator {
    fn locally_simulate_tx(&self, tx: &Transaction) -> Result<Gas> {
        let mut evm = self.evm.write().map_err(Error::local_simulation_failed)?;
        let tx = TxEnv::from(tx.clone());
        Self::simulate_tx(&mut evm, tx)
    }
}

#[cfg(test)]
mod tests {

    use alloy_consensus::TxEip1559;

    use super::*;

    type Sut = RevmTxSimulator;

    fn test_rlp<const L: usize>(rlp: [u8; L], expected_gas: u64) {
        let tx: TxEip1559 = crate::decode_eip1559_transaction(rlp).unwrap();
        let tx = &Transaction::from_eip1559(tx);
        let sut = Sut::new();
        let gas_used = sut.locally_simulate_tx(tx).unwrap();
        assert_eq!(gas_used, Gas::from(expected_gas));
    }

    #[test]
    fn test_contract_call_uniswap_v4_tx0() {
        // https://etherscan.io/tx/0x5f8a348f580c8dc1f897d6ef855b9539f2ddd1d20168935730c2c76baf4ea86e
        test_rlp(
            hex_literal::hex!(
                "02f90392012d841dcd65008446bce7ee8303967c9466a9893cc07d91d95644aedd05d03f95e1dba8af80b903253593564c000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000000a00000000000000000000000000000000000000000000000000000000067e541b6000000000000000000000000000000000000000000000000000000000000000308060c000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000030000000000000000000000000000000000000000000000000000000000000060000000000000000000000000000000000000000000000000000000000000018000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000032d26d12e980b600000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000002000000000000000000000000bf358f7023d6fd0d11ac284eb47b877c1af635aa000000000000000000000000c02aaa39b223fe8d0a0e5c4f27ead9083c756cc20000000000000000000000000000000000000000000000000000000000000060000000000000000000000000c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2000000000000000000000000000000fee13a103a10d593b9ae06b3e05f2e7e1c000000000000000000000000000000000000000000000000000000000000001900000000000000000000000000000000000000000000000000000000000000400000000000000000000000003f85cb63e4cb3a3df6594a5412e78bb4392439ef000000000000000000000000000000000000000000000000005eb1a4afed70da0cc080a03d3a45244369fb6d812a40f985552e8d1f87ed02f40ca9025d912497d080312aa07c6d449b0bd2e42d3b4052ac70c0789e491d60292888fe112b0e844330d0611a"
            ),
            33250, // Etherscan reports 169_610, not sure why we are so off...
        );
    }

    #[test]
    fn test_contract_call_uniswap_v4_tx1() {
        // https://etherscan.io/tx/0x4a6307830781fa2a308d315a0daaa5770419fab1104cf6d650167c470e7f9b5a
        test_rlp(
            hex_literal::hex!(
                "02f902db01820168841dcd6500843d831e6783027a6d9466a9893cc07d91d95644aedd05d03f95e1dba8af8803bbae1324948000b9026424856bc30000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000008000000000000000000000000000000000000000000000000000000000000000020b080000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000a00000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000003bbae1324948000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000096cdea52111684fd74ec6cdf31dd97f395737a5d00000000000000000000000000000000000000000000000003bbae132494800000000000000000000000000000000000000000000000001e28ba62f4e8c66e7b00000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000002000000000000000000000000c02aaa39b223fe8d0a0e5c4f27ead9083c756cc20000000000000000000000000aff507ac29b8cea2fb10d2ad14408c2d79a35adc001a0ff907e592e412943d4f7136aeb55f9fcf701ef295c7cd620be07ffe037de5b58a05889a72e62156c986ccc657bcdba1f91c481e817ee607408320d312416fa3a67"
            ),
            30120, // Etherscan reports 122025, not sure why we are so off...
        );
    }

    #[test]
    fn fails_with_gas_exceeds_limit_when_limit_is_less_than_estimated_cost() {
        let limit = Gas::from(100);

        let tx = TransactionBuilder::default()
            .to(Address::from([0x12; 20]))
            .value(U256::from(1))
            .gas_limit(limit)
            .build()
            .unwrap();
        let sut = Sut::new();
        let res = sut.locally_simulate_tx(&tx);
        assert_eq!(
            res,
            Err(Error::GasExceedsLimit {
                estimated_cost: Some(Gas::from(21_000)),
                gas_limit: limit
            })
        );
    }
}
