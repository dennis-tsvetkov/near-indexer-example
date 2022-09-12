use actix;

use anyhow::Result;
use clap::Parser;
use tokio::sync::mpsc;
use tracing::info;

use configs::{Opts, SubCommand};
use near_indexer;

mod configs;

async fn listen_blocks(mut stream: mpsc::Receiver<near_indexer::StreamerMessage>) {
    while let Some(streamer_message) = stream.recv().await {
        info!(
            target: "indexer_example",
            "#{} {} Shards: {}, Transactions: {}, Receipts: {}, ExecutionOutcomes: {}",
            streamer_message.block.header.height,
            streamer_message.block.header.hash,
            streamer_message.shards.len(),
            streamer_message.shards.iter().map(|shard| if let Some(chunk) = &shard.chunk { chunk.transactions.len() } else { 0usize }).sum::<usize>(),
            streamer_message.shards.iter().map(|shard| if let Some(chunk) = &shard.chunk { chunk.receipts.len() } else { 0usize }).sum::<usize>(),
            streamer_message.shards.iter().map(|shard| shard.receipt_execution_outcomes.len()).sum::<usize>(),
        );
    }
}

fn main() -> Result<()> {
    // We use it to automatically search the for root certificates to perform HTTPS calls
    // (sending telemetry and downloading genesis)
    openssl_probe::init_ssl_cert_env_vars();
    let env_filter = near_o11y::tracing_subscriber::EnvFilter::new(
        "nearcore=info,indexer_example=info,tokio_reactor=info,near=info,\
         stats=info,telemetry=info,indexer=info,near-performance-metrics=info",
    );
    let _susbcriber = near_o11y::default_subscriber(env_filter).global();
    let opts: Opts = Opts::parse();

    let home_dir =
        opts.home_dir.unwrap_or(std::path::PathBuf::from(near_indexer::get_default_home()));

    match opts.subcmd {
        SubCommand::Run => {
            let indexer_config = near_indexer::IndexerConfig {
                home_dir,
                sync_mode: near_indexer::SyncModeEnum::FromInterruption,
                await_for_node_synced: near_indexer::AwaitForNodeSyncedEnum::WaitForFullSync,
            };
            let system = actix::System::new();
            system.block_on(async move {
                let indexer = near_indexer::Indexer::new(indexer_config).expect("Indexer::new()");
                let stream = indexer.streamer();
                actix::spawn(listen_blocks(stream));
            });
            system.run()?;
        }
        SubCommand::Init(config) => near_indexer::indexer_init_configs(&home_dir, config.into())?,
    }
    Ok(())
}
