use crate::prelude::*;

use alloy_consensus::transaction::RlpEcdsaDecodableTx;
use alloy_consensus::{Signed, TxEip1559};

pub fn decode_eip1559_transaction(raw_tx: impl AsRef<[u8]>) -> Result<TxEip1559, Error> {
    if let Ok(signed_tx) = _decode_eip1559_transaction_signed(raw_tx.as_ref()) {
        Ok(signed_tx.tx().clone())
    } else {
        _decode_eip1559_transaction_not_signed(raw_tx)
    }
}

fn _decode_eip1559_transaction_not_signed(raw_tx: impl AsRef<[u8]>) -> Result<TxEip1559, Error> {
    let mut buf = raw_tx.as_ref();
    TxEip1559::rlp_decode(&mut buf).map_err(Error::decode_rlp_decode_bytes_into_eip1559)
}

fn _decode_eip1559_transaction_signed(
    raw_tx: impl AsRef<[u8]>,
) -> Result<Signed<TxEip1559>, Error> {
    let mut buf = raw_tx.as_ref();
    if buf.starts_with(&[0x02]) {
        buf = &buf[1..];
    }
    TxEip1559::rlp_decode_signed(&mut buf)
        .map_err(Error::decode_rlp_decode_bytes_into_signed_eip1559)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;
    #[test]
    fn decode_rlp() {
        // https://etherscan.io/tx/0xb1869db00d08d706059ae6a167b9d89b01884606ee4dec42c19c9c6466471542
        let raw_tx_signed = hex!(
            "02f87201824f4c83142ebf842d441366825208942e575fe17124f7ef2d22bbfb33cf3dbfc3f002d68711c37937e0800080c001a0152c51f0aa71d7698b486a34f8ffc9b61cc7a000c34d48e1cf9361d8973ba518a024216a87cb193b7e502ad9ddbcfc9674c40fe98bd4a7bda575ba03185621cd13"
        );

        let tx = decode_eip1559_transaction(raw_tx_signed).unwrap();

        let raw_tx_not_signed = hex!(
            "ef01824f4c83142ebf842d441366825208942e575fe17124f7ef2d22bbfb33cf3dbfc3f002d68711c37937e0800080c0"
        );
        let tx2 = decode_eip1559_transaction(raw_tx_not_signed).unwrap();

        assert_eq!(tx, tx2)
    }
}
