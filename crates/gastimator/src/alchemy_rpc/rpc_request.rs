use crate::prelude::*;

#[derive(Serialize, Builder, Getters)]
#[builder(setter(into))]
pub struct RpcRequest<Params: Serialize> {
    #[builder(default = "2.0".to_string())]
    jsonrpc: String,
    method: String,
    params: Vec<Params>,
    id: u64,
}
