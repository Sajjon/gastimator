use crate::prelude::*;

/// A "raw" transaction, with [`RLP`][rlp] encoding.
///
/// [rlp]: https://ethereum.org/en/developers/docs/data-structures-and-encoding/rlp/
#[derive(Debug, Clone, derive_more::Display, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[display("{}", hex::encode(&self.rlp))]
pub struct RawTransaction {
    /// The RLP encoded transaction.
    pub rlp: Bytes,
}

impl std::str::FromStr for RawTransaction {
    type Err = crate::Error;

    /// Tries to decode the string `s` into a `BagOfBytes`. Will fail
    /// if the string is not valid hex or if the decoded bytes does
    /// not have length 32.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        hex::decode(s)
            .map_err(|_| crate::Error::StringNotHex {
                bad_value: s.to_owned(),
            })
            .map(Bytes::from)
            .map(|rlp| Self { rlp })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn deserialize() {
        let json = json!({"rlp": "dead"});
        let raw: RawTransaction = serde_json::from_value(json).unwrap();
        assert_eq!(
            raw,
            RawTransaction {
                rlp: hex_literal::hex!("dead").into()
            }
        );
    }
}
