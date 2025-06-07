use alloy::providers::Provider;
use eyre::Result;
use tracing::{info, debug};

use loom::core::topology::{Topology, TopologyConfig};
use loom::evm::db::LoomDBType;
use loom::types::events::MarketEvents;
use loom::defi::address_book::PancakeV3PoolAddress;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("debug"),
    )
    .format_timestamp_micros()
    .init();

    let topology_config = TopologyConfig::load_from_file("config.toml".to_string())?;
    
    let topology =
        Topology::<LoomDBType>::from_config(topology_config)
            .build_blockchains()
            .start_clients()
            .await?;

    let client = topology.get_client(Some("remote".to_string()).as_ref())?;
    let blockchain = topology.get_blockchain(Some("mainnet".to_string()).as_ref())?;
    let blockchain_state = topology.get_blockchain_state(Some("mainnet".to_string()).as_ref())?;
    
    let pancake_v3_pool = PancakeV3PoolAddress::USDC_USDT_100;

    let block_nr = client.get_block_number().await?;
    info!("Starting at block: {}", block_nr);
    info!("Monitoring PancakeV3 pool USDC/USDT at address: {}", pancake_v3_pool);

    // Subscribe to market events
    let mut market_events = blockchain.market_events_channel().subscribe();
    
    loop {
        if let Ok(event) = market_events.recv().await {
            match event {
                MarketEvents::BlockLogsUpdate { block_number, block_hash } => {
                    let market_guard = blockchain.market().read().await;
                    if let Some(pool) = market_guard.get_pool_by_address(&pancake_v3_pool) {
                        debug!("Block {} processing swap events for pool {}", block_number, pool.get_address());
                        let swap_directions = pool.get_swap_directions();
                        info!("Available swap directions: {:?}", swap_directions);
                    }
                }
                MarketEvents::NewPoolLoaded { pool_id, swap_path_idx_vec } => {
                    if let Ok(addr) = pool_id.address() {
                        if addr == pancake_v3_pool {
                            info!("PancakeV3 USDC/USDT pool loaded, paths: {:?}", swap_path_idx_vec);
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
