use crate::prelude::*;

pub trait IsRpcRequest {
    type Param: Serialize;
    fn method() -> String;
}
