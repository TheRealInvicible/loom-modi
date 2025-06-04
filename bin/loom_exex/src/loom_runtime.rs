use alloy::network::Ethereum;
use alloy::primitives::Address;
use alloy::providers::Provider;
use axum::Router;
use eyre::{ErrReport, OptionExt};
use loom::core::blockchain::{Blockchain, BlockchainState, Strategy};
use loom_types_entities::pool_config::PoolsLoadingConfig;
use loom::core::blockchain_actors::BlockchainActors;
use loom::core::topology::{BroadcasterConfig, EncoderConfig, TopologyConfig};
use loom::evm::db::DatabaseLoomExt;
use loom::execution::multicaller::MulticallerSwapEncoder;
use loom::node::actor_config::NodeBlockActorConfig;
use loom::node::debug_provider::DebugProviderExt;
use loom::node::exex::loom_exex;
use loom::storage::db::init_db_pool;
use loom::strategy::backrun::{BackrunConfig, BackrunConfigSection};
use loom::types::entities::strategy_config::load_from_file;
use loom::types::entities::{BlockHistoryState, PoolClass};
use reth::api::NodeTypes;
use reth::revm::{Database, DatabaseCommit, DatabaseRef};
use reth_exex::ExExContext;
use reth_node_api::FullNodeComponents;
use reth_primitives::EthPrimitives;
use std::env;
use std::future::Future;
use tracing::info;

pub async fn init<Node>(
    ctx: ExExContext<Node>,
    bc: Blockchain,
    config: NodeBlockActorConfig,
) -> eyre::Result<impl Future<Output = eyre::Result<()>>>
where
    Node: FullNodeComponents<Types: NodeTypes<Primitives = EthPrimitives>>,
{
    Ok(loom_exex(ctx, bc, config.clone()))
}

pub async fn start_loom<P, DB>(
    provider: P,
    bc: Blockchain,
    bc_state: BlockchainState<DB>,
    strategy: Strategy<DB>,
    topology_config: TopologyConfig,
    loom_config_filepath: String,
    is_exex: bool,
) -> eyre::Result<()>
where
    P: Provider<Ethereum> + DebugProviderExt<Ethereum> + Send + Sync + Clone + 'static,
    DB: Database<Error = ErrReport>
        + DatabaseRef<Error = ErrReport>
        + DatabaseCommit
        + DatabaseLoomExt
        + BlockHistoryState
        + Send
        + Sync
        + Clone
        + Default
        + 'static,
{
    let chain_id = provider.get_chain_id().await?;

    info!(chain_id = ?chain_id, "Starting Loom" );

    let (_encoder_name, encoder) = topology_config.encoders.iter().next().ok_or_eyre("NO_ENCODER")?;

    let multicaller_address: Option<Address> = match encoder {
        EncoderConfig::SwapStep(e) => e.address.parse().ok(),
    };
    let multicaller_address = multicaller_address.ok_or_eyre("MULTICALLER_ADDRESS_NOT_SET")?;
    let private_key_encrypted = hex::decode(env::var("DATA")?)?;
    info!(address=?multicaller_address, "Multicaller");

    let webserver_host = topology_config.webserver.unwrap_or_default().host;
    let db_url = topology_config.database.unwrap().url;
    let db_pool = init_db_pool(db_url).await?;

    let relays = topology_config
        .actors
        .broadcaster
        .as_ref()
        .and_then(|b| b.get("mainnet"))
        .map(|b| match b {
            BroadcasterConfig::Flashbots(f) => f.relays(),
        })
        .unwrap_or_default();

    let pools_config = PoolsLoadingConfig::new().disable_all().enable(PoolClass::UniswapV2);

    let backrun_config: BackrunConfigSection = load_from_file::<BackrunConfigSection>(loom_config_filepath.into()).await?;
    let backrun_config: BackrunConfig = backrun_config.backrun_strategy;

    let swap_encoder = MulticallerSwapEncoder::default_with_address(multicaller_address);

    let mut bc_actors = BlockchainActors::new(provider.clone(), swap_encoder.clone(), bc.clone(), bc_state, strategy, relays);
    bc_actors
        .mempool()?
        .with_wait_for_node_sync()?
        .initialize_signers_with_encrypted_key(private_key_encrypted)?
        .with_block_history()?
        .with_price_station()?
        .with_health_monitor_pools()?
        .with_health_monitor_stuffing_tx()?
        .with_swap_encoder(swap_encoder)?
        .with_evm_estimator()?
        .with_signers()?
        .with_flashbots_broadcaster(true)?
        .with_market_state_preloader()?
        .with_nonce_and_balance_monitor()?
        .with_pool_history_loader(pools_config.clone())?
        .with_new_pool_loader(pools_config.clone())?
        .with_pool_loader(pools_config.clone())?
        .with_swap_path_merger()?
        .with_diff_path_merger()?
        .with_same_path_merger()?
        .with_backrun_block(backrun_config.clone())?
        .with_backrun_mempool(backrun_config)?
        .with_web_server(webserver_host, Router::new(), db_pool)?;
    if !is_exex {
        bc_actors.with_block_events(NodeBlockActorConfig::all_enabled())?.with_remote_mempool(provider.clone())?;
    }

    if let Some(influxdb_config) = topology_config.influxdb {
        bc_actors
            .with_influxdb_writer(influxdb_config.url, influxdb_config.database, influxdb_config.tags)?
            .with_block_latency_recorder()?;
    }

    bc_actors.wait().await;

    Ok(())
}
