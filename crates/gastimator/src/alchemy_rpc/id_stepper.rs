use crate::prelude::*;

/// A helper which generates unique request IDs for each JSON-RPC request.
#[derive(Default)]
pub struct IdStepper(RwLock<u64>);
impl IdStepper {
    /// Returns the next request ID, e.g. for a JSON-RPC request.
    pub fn next(&self) -> u64 {
        let mut id = self.0.write().unwrap();
        *id += 1;
        *id
    }
}
