use crate::prelude::*;

#[derive(Default)]
pub struct IdStepper(RwLock<u64>);
impl IdStepper {
    pub fn next(&self) -> u64 {
        let mut id = self.0.write().unwrap();
        *id += 1;
        *id
    }
}
