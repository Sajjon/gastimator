use crate::prelude::*;

#[async_trait::async_trait]
pub trait RemoteGasEstimator {
    async fn estimate_gas(&self, tx: &Transaction) -> Result<Gas>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uint256_from_hex() {
        let sut = U256::from_str_radix(&"0x5208"[2..], 16).unwrap();
        assert_eq!(sut, U256::from(21000));
    }
}
