//! Binary to assert specific behavior in the World Chain devnet
use core::time;
use std::{
    process::Command,
    sync::Arc,
    time::{Duration, Instant},
};

use alloy_primitives::Bytes;
use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_types_eth::BlockNumberOrTag;
use alloy_transport::Transport;
use clap::Parser;
use eyre::eyre::{eyre, Result};
use serde::Deserialize;
use tokio::time::sleep;
use tracing::{debug, info};

const PBH_FIXTURE: &str = include_str!("../../../../devnet/fixtures/fixture.json");

#[derive(Deserialize, Clone)]
pub struct PbhFixture {
    pub fixture: Vec<Bytes>,
}

#[derive(Parser)]
pub struct Args {
    /// Build a PBH block from transaction fixtures with the given number of transactions
    #[clap(short, long, conflicts_with = "fallback")]
    pub build: bool,
    /// Run a Fallback test
    #[clap(short, long, conflicts_with = "build")]
    pub fallback: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let args = Args::parse();

    // Grab the ports
    let builder_socket = run_command(
        "kurtosis",
        &[
            "port",
            "print",
            "world-chain",
            "wc-admin-world-chain-builder",
            "rpc",
        ],
    )?;

    info!("Builder socket: {}", builder_socket);

    let sequencer_socket = run_command(
        "kurtosis",
        &["port", "print", "world-chain", "wc-admin-op-geth", "rpc"],
    )?;

    info!("Sequencer socket: {}", sequencer_socket);

    let sequencer_provider =
        Arc::new(ProviderBuilder::default().on_http(sequencer_socket.parse().unwrap()));
    let builder_provider =
        Arc::new(ProviderBuilder::default().on_http(builder_socket.parse().unwrap()));

    let timeout = std::time::Duration::from_secs(30);

    info!("Waiting for the devnet");

    let f = async {
        let wait_0 = wait(sequencer_provider.clone(), timeout);
        let wait_1 = wait(builder_provider.clone(), timeout);
        tokio::join!(wait_0, wait_1);
    };
    f.await;

    info!("Devnet is ready");

    if args.build {
        info!("Running block building test");
        assert_build(builder_provider).await?;
    }

    if args.fallback {
        info!("Running Sequencer fallback test");
        assert_fallback(sequencer_provider).await?;
    }

    Ok(())
}

async fn wait<T, P>(provider: P, timeout: time::Duration)
where
    T: Transport + Clone,
    P: Provider<T>,
{
    let start = Instant::now();
    loop {
        if let Ok(_) = provider
            .get_block_by_number(BlockNumberOrTag::Latest, false)
            .await
        {
            break;
        }
        sleep(Duration::from_secs(1)).await;
        if start.elapsed() > timeout {
            panic!("Timeout waiting for the devnet");
        }
    }
}

pub async fn assert_build<T, P>(builder_provider: Arc<P>) -> Result<()>
where
    T: Transport + Clone,
    P: Provider<T>,
{
    let fixture = serde_json::from_str::<PbhFixture>(PBH_FIXTURE)?;
    let mut queue = vec![];
    for transaction in fixture.fixture.iter() {
        let tx = builder_provider.send_raw_transaction(transaction).await?;
        queue.push(tx);
    }

    let futs = queue.into_iter().map(|builder| async {
        let hash = builder.tx_hash().clone();
        let receipt = builder.get_receipt().await;
        assert!(receipt.is_ok());
        debug!(receipt = ?receipt.unwrap(), hash = ?hash, "Transaction Receipt Received");
    });

    futures::future::join_all(futs).await;

    Ok(())
}

pub async fn assert_fallback<T, P>(sequencer_provider: P) -> Result<()>
where
    T: Transport + Clone,
    P: Provider<T>,
{
    // Grab the latest block number
    let block_number = sequencer_provider.get_block_number().await?;
    // Take the Builder service down.
    run_command(
        "kurtosis",
        &[
            "service",
            "stop",
            "world-chain",
            "wc-admin-world-chain-builder",
        ],
    )?;
    sleep(Duration::from_secs(3)).await;

    // Assert the chain has progressed
    let new_block_number = sequencer_provider.get_block_number().await?;
    assert!(new_block_number > block_number);
    // Restart the service
    run_command(
        "kurtosis",
        &[
            "service",
            "start",
            "world-chain",
            "wc-admin-world-chain-builder",
        ],
    )?;
    Ok(())
}

pub fn run_command(cmd: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(cmd).args(args).output()?;
    if output.status.success() {
        Ok(String::from_utf8(output.stdout)?)
    } else {
        Err(eyre!(
            "Command failed: {:?}",
            String::from_utf8(output.stdout).unwrap(),
        ))
    }
}
