use crate::prelude::*;

// ========================================
// Private
// ========================================

// Uh... axum needs this. I can probably impl Handler for Gastimator instead.
// but seems not worth it for now.
async fn estimate_gas(
    Json(tx): Json<Transaction>,
    gastimator: Arc<Gastimator>,
) -> Result<Json<GasEstimateResponse>> {
    gastimator.estimate_gas(tx).await.map(Json)
}

// Uh... axum needs this. I can probably impl Handler for Gastimator instead.
// but seems not worth it for now.
async fn estimate_gas_rlp(
    Json(tx): Json<RawTransaction>,
    gastimator: Arc<Gastimator>,
) -> Result<Json<GasEstimateResponse>> {
    estimate_gas(Json(Transaction::try_from(tx)?), gastimator).await
}

fn init_logging() {
    pretty_env_logger::init();
}

fn build_app(gastimator: Arc<Gastimator>) -> Router {
    Router::new()
        .route("/tx", {
            let gastimator = gastimator.clone();
            post(move |body| estimate_gas(body, gastimator))
        })
        .route("/rlp", {
            let gastimator = gastimator.clone();
            post(move |body| estimate_gas_rlp(body, gastimator))
        })
}

async fn bind_and_signal(
    address: String,
    ready_tx: oneshot::Sender<SocketAddr>,
) -> Result<(tokio::net::TcpListener, SocketAddr)> {
    let listener = tokio::net::TcpListener::bind(&address)
        .await
        .map_err(Error::bind)?;
    let bound_addr = listener.local_addr().map_err(Error::get_bound_address)?;
    ready_tx
        .send(bound_addr)
        .map_err(|_| Error::FailedToSignalReadiness)?;
    Ok((listener, bound_addr))
}

// ========================================
// Public
// ========================================

/// Starts the server and signals readiness when the endpoints are live.
pub async fn run_signaling_readiness(
    config: &Config,
    ready_tx: oneshot::Sender<SocketAddr>,
) -> Result<()> {
    init_logging();
    debug!("Starting gastimate server... args: {:?}", config.server());
    let gastimator = Arc::new(Gastimator::new(config.alchemy_api_key().clone()));
    let app = build_app(gastimator);
    let (listener, address) =
        bind_and_signal(config.server().address_with_port(), ready_tx).await?;
    info!("Listening on: {}", address);
    axum::serve(listener, app).await.map_err(Error::start)
}

pub async fn run(config: &Config) {
    let config = config.clone();
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
    let server_handle =
        tokio::spawn(async move { run_signaling_readiness(&config, ready_tx).await });
    // Wait for the server to signal readiness and get the bound address
    ready_rx
        .await
        .map_err(|_| Error::FailedToSignalReadiness)
        .unwrap_display();
    let _ = server_handle
        .into_future()
        .await
        .expect("Should never finish");
}
