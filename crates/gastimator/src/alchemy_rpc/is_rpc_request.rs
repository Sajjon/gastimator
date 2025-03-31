use crate::prelude::*;

/// A trait for types that can be used as RPC requests.
///
/// Returns the method name and the parameter type.
pub trait IsRpcRequest {
    /// The input parameter type for the RPC method.
    type Param: Serialize;
    /// The RPC method name.
    fn method() -> String;
}
