use crate::prelude::*;

#[derive(Deserialize, Debug)]
pub struct RpcResponse {
    pub result: String,
}

impl RpcResponse {
    pub fn result_strip_0x(&self) -> String {
        if self.result.starts_with("0x") {
            self.result[2..].to_string()
        } else {
            self.result.clone()
        }
    }
}
