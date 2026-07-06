use clap::Parser;
use std::time::Duration;
use tokio::time;
use tracing::{error, info, warn};

mod relay;
mod telemetry;

#[derive(Parser, Debug)]
#[command(name = "energy-daemon")]
struct Args {
    #[arg(long, env = "STELLAR_RPC_URL")]
    stellar_rpc_url: String,

    #[arg(long, env = "CONTRACT_ID")]
    contract_id: String,

    #[arg(long, env = "HARDWARE_GATEWAY_KEY")]
    hardware_gateway_key: String,

    #[arg(long, default_value = "60")]
    poll_interval_secs: u64,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "energy_daemon=info".into()),
        )
        .init();

    dotenvy::dotenv().ok();
    let args = Args::parse();

    info!(
        "energy-daemon starting — polling every {}s",
        args.poll_interval_secs
    );

    let mut telemetry = telemetry::SimulatedTelemetry::new(100);
    let mut relay = relay::RelaySimulator::new();

    let mut interval = time::interval(Duration::from_secs(args.poll_interval_secs));
    loop {
        interval.tick().await;
        info!("--- tick ---");

        let watt_hours = telemetry.sample();
        info!("telemetry: {watt_hours} Wh consumed since last tick");

        match check_stream_health(&args).await {
            Ok(true) => {
                info!("stream is active — relay remains ON");
                relay.keep_on();
            }
            Ok(false) => {
                warn!("stream is dry — cutting relay OFF");
                relay.cut_off();
            }
            Err(e) => {
                error!("failed to query stream health: {e}");
            }
        }
    }
}

async fn check_stream_health(args: &Args) -> Result<bool, reqwest::Error> {
    let client = reqwest::Client::new();
    let resp = client
        .post(&args.stellar_rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "simulateTransaction",
            "params": {
                "transaction": format!(
                    "AAAABQAAAAAgQECkpLespG+={}",
                    args.hardware_gateway_key
                ),
                "resourceConfig": {
                    "instructionLeeway": 1000000
                }
            }
        }))
        .send()
        .await?;

    if resp.status().is_success() {
        Ok(true)
    } else {
        Ok(false)
    }
}
