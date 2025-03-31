mod cli;
use cli::*;
use gastimator_rest::run;

#[tokio::main]
async fn main() {
    let args = Cli::parse();
    let config = &Config::try_from(args).unwrap_display();
    run(config).await;
}
