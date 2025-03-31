use crate::prelude::*;

// ========================================
// Private
// ========================================

// Uh... axum needs this. I can probably impl Handler for Gastimator instead.
// but seems not worth it for now.
async fn estimate_gas_canonical(
    Json(tx): Json<Transaction>,
    gastimator: Arc<Gastimator>,
) -> Result<Json<GasEstimateResponse>> {
    Gastimator::estimate_gas_canonical(gastimator, tx)
        .await
        .map(Json)
}

// Uh... axum needs this. I can probably impl Handler for Gastimator instead.
// but seems not worth it for now.
async fn estimate_gas_rlp(
    Json(tx): Json<RawTransaction>,
    gastimator: Arc<Gastimator>,
) -> Result<Json<GasEstimateResponse>> {
    let tx = Transaction::try_from(tx)?;
    Gastimator::estimate_gas_canonical(gastimator, tx)
        .await
        .map(Json)
}

// ========================================
// Public
// ========================================

/// Starts the server and signals readiness when the endpoints are live, using
/// the `ready_tx` channel.
pub async fn run_signaling_readiness(
    config: &Config,
    ready_tx: oneshot::Sender<SocketAddr>,
) -> Result<()> {
    pretty_env_logger::init();
    debug!("Starting gastimate server... args: {:?}", config.server());

    let gastimator = Arc::new(Gastimator::new(config.alchemy_api_key().clone()));

    // build our application with a single route
    let app = Router::new()
        .route("/tx", {
            let gastimator = gastimator.clone();
            post(move |body| estimate_gas_canonical(body, gastimator))
        })
        .route("/rlp", {
            let gastimator = gastimator.clone();
            post(move |body| estimate_gas_rlp(body, gastimator))
        });

    let address = config.server().address_with_port();
    let listener = tokio::net::TcpListener::bind(&address)
        .await
        .map_err(Error::bind)?;

    let address = listener.local_addr().map_err(Error::get_bound_address)?;
    info!("Listening on: {}", address);
    // Signal that the server is ready with the bound address
    ready_tx
        .send(address)
        .map_err(|_| Error::FailedToSignalReadiness)?;

    axum::serve(listener, app).await.map_err(Error::start)?;
    Ok(())
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
