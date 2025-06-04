use alloy::primitives::ChainId;
use alloy_chains::{Chain, NamedChain};
use eyre::{eyre, OptionExt, Result};
use loom_defi_address_book::TokenAddressArbitrum;
use loom_types_entities::{Market, Token};
use loom::core::actors::{Accessor, Actor, Consumer, Producer};
use loom::core::router::SwapRouterActor;
use loom::core::topology::{Topology, TopologyConfig};
use loom::defi::health_monitor::{MetricsRecorderActor, StateHealthMonitorActor, StuffingTxMonitorActor};
use loom::evm::db::LoomDBType;
use loom::execution::multicaller::MulticallerSwapEncoder;
use loom::metrics::InfluxDbWriterActor;
use loom::strategy::backrun::{BackrunConfig, BackrunConfigSection, StateChangeArbActor};
use loom::strategy::merger::{ArbSwapPathMergerActor, DiffPathMergerActor, SamePathMergerActor};
use loom::types::entities::strategy_config::load_from_file;
use loom::types::events::MarketEvents;



#[tokio::main]
async fn main() {

    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("debug,tokio_tungstenite=off,tungstenite=off,alloy_rpc_client=off"),
    )
    .format_timestamp_micros()
    .init();

    let topology_config = TopologyConfig::load_from_file("statechangeconfig.toml".to_string())?;
    let influxdb_config = topology_config.influxdb.clone();

    let encoder = MulticallerSwapEncoder::default();

    let topology =
        Topology::<LoomDBType>::from_config(topology_config).with_swap_encoder(encoder).build_blockchains().start_clients().await?;

    let mut worker_task_vec = topology.start_actors().await?;

    let client = topology.get_client(Some("remote".to_string()).as_ref())?;
    let blockchain = topology.get_blockchain(Some("mainnet".to_string()).as_ref())?;
    let blockchain_state = topology.get_blockchain_state(Some("mainnet".to_string()).as_ref())?;
    let strategy = topology.get_strategy(Some("mainnet".to_string()).as_ref())?;

    let tx_signers = topology.get_signers(Some("env_signer".to_string()).as_ref())?;

    let backrun_config: BackrunConfigSection = load_from_file("./statechangeconfig.toml".to_string().into()).await?;
    let backrun_config: BackrunConfig = backrun_config.backrun_strategy;

    let block_nr = client.get_block_number().await?;
    info!("Block : {}", block_nr);

    info!("Creating shared state");

    info!("Starting state change arb actor");
    let mut state_change_arb_actor = StateChangeArbActor::new(client.clone(), true, true, backrun_config);
    match state_change_arb_actor
        .access(blockchain.mempool())
        .access(blockchain.latest_block())
        .access(blockchain.market())
        .access(blockchain_state.market_state())
        .access(blockchain_state.block_history())
        .consume(blockchain.market_events_channel())
        .consume(blockchain.mempool_events_channel())
        .produce(strategy.swap_compose_channel())
        .produce(blockchain.health_monitor_channel())
        .produce(blockchain.influxdb_write_channel())
        .start()
    {
        Err(e) => {
            error!("{}", e)
        }
        Ok(r) => {
            worker_task_vec.extend(r);
            info!("State change arb actor started successfully")
        }
    }




    // listening to MarketEvents in an infinite loop
    let mut s = blockchain.market_events_channel().subscribe();
    loop {
        let msg = s.recv().await;
        if let Ok(msg) = msg {
            match msg {
                MarketEvents::BlockTxUpdate { block_number, block_hash } => {
                    info!("New block received {} {}", block_number, block_hash);
                }
                MarketEvents::BlockStateUpdate { block_hash } => {
                    info!("New block state received {}", block_hash);
                }
                _ => {}
            }
        }
    }

}
