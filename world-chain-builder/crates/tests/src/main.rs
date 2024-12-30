//! Devnet Deployment Binary
//!     - Starts the Kurtosis Devnet
//!     - Deploys the 4337 PBH Contract Stack
//!     - Asserts all Services are running, and healthy
//! NOTE: This binary assumes that the Kurtosis Devnet is not running, and the `world-chain` enclave has been cleaned.

use std::{
    env,
    path::Path,
    sync::Arc,
    time::{self, Duration, Instant},
};

use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_types_eth::BlockNumberOrTag;
use alloy_transport::Transport;
use eyre::eyre::{eyre, Result};
use fixtures::generate_test_fixture;
use std::process::Command;
use tokio::time::sleep;
use tracing::info;
pub mod cases;
pub mod fixtures;

const DEPLOYER_DEV: &str = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";

#[tokio::main]
async fn main() -> Result<()> {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }

    tracing_subscriber::fmt::init();

    let (builder_rpc, sequencer_rpc) = start_devnet().await?;

    let sequencer_provider =
        Arc::new(ProviderBuilder::default().on_http(sequencer_rpc.parse().unwrap()));
    let builder_provider =
        Arc::new(ProviderBuilder::default().on_http(builder_rpc.parse().unwrap()));

    let timeout = std::time::Duration::from_secs(30);

    info!("Waiting for the devnet to be ready");

    let f = async {
        let wait_0 = wait(sequencer_provider.clone(), timeout);
        let wait_1 = wait(builder_provider.clone(), timeout);
        tokio::join!(wait_0, wait_1);
    };
    f.await;

    info!("Deploying contracts");
    deploy_contracts(builder_rpc.clone()).await?;

    info!("Generating test fixtures");
    let fixture = generate_test_fixture().await;

    info!("Running block building test");
    cases::assert_build(builder_provider.clone(), fixture).await?;
    info!("Running fallback test");
    cases::assert_fallback(sequencer_provider.clone()).await?;

    Ok(())
}

async fn start_devnet() -> Result<(String, String)> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .unwrap()
        .canonicalize()?;

    run_command(&"just", &["devnet-up"], path).await?;

    let builder_socket = run_command(
        "kurtosis",
        &[
            "port",
            "print",
            "world-chain",
            "wc-admin-world-chain-builder",
            "rpc",
        ],
        env!("CARGO_MANIFEST_DIR"),
    )
    .await?;

    let builder_socket = format!(
        "http://{}",
        builder_socket.split("http://").collect::<Vec<&str>>()[1]
    );

    let sequencer_socket = run_command(
        "kurtosis",
        &["port", "print", "world-chain", "wc-admin-op-geth", "rpc"],
        env!("CARGO_MANIFEST_DIR"),
    )
    .await?;

    let sequencer_socket = format!(
        "http://{}",
        sequencer_socket.split("http://").collect::<Vec<&str>>()[1]
    );

    info!(
        builder_rpc = %builder_socket,
        sequencer_rpc = %sequencer_socket,
        "Devnet is ready"
    );

    Ok((builder_socket, sequencer_socket))
}

async fn deploy_contracts(builder_rpc: String) -> Result<()> {
    if std::env::var("PRIVATE_KEY").is_err() {
        std::env::set_var("PRIVATE_KEY", DEPLOYER_DEV);
    }
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .unwrap()
        .join("contracts/")
        .canonicalize()?;

    info!("Deploying PBH 4337 contracts at {:?}", builder_rpc);

    run_command(
        "forge",
        &[
            "script",
            "scripts/DeployDevnet.s.sol:DeployDevnet",
            "--rpc-url",
            &builder_rpc,
            "--broadcast",
        ],
        path,
    )
    .await?;
    Ok(())
}

async fn wait<T, P>(provider: P, timeout: time::Duration)
where
    T: Transport + Clone,
    P: Provider<T>,
{
    let start = Instant::now();
    loop {
        if provider
            .get_block_by_number(BlockNumberOrTag::Latest, false)
            .await
            .is_ok()
        {
            break;
        }
        sleep(Duration::from_secs(1)).await;
        if start.elapsed() > timeout {
            panic!("Timeout waiting for the devnet");
        }
    }
}

pub async fn run_command(cmd: &str, args: &[&str], ctx: impl AsRef<Path>) -> Result<String> {
    let output = Command::new(cmd).current_dir(ctx).args(args).output()?;
    if output.status.success() {
        let stdout = String::from_utf8(output.stdout)?;
        info!("{:?}", stdout.trim_end_matches("/n"));
        Ok(stdout)
    } else {
        Err(eyre!(
            "Command failed: {:?}",
            String::from_utf8(output.stdout)?.trim_end_matches("/n"),
        ))
    }
}
