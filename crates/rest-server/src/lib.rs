mod server;

pub mod prelude {

    // INTERNAL MODULES
    pub use crate::server::*;

    // INTERNAL CRATES
    pub use gastimator::prelude::*;

    // EXTERNAL CRATES
    pub use axum::{Json, Router, response::IntoResponse, routing::post};
    pub use tokio::sync::oneshot;
}

pub use prelude::*;
