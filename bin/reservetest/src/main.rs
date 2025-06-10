use std::env;

use alloy::{
    network::Ethereum,
    primitives::{address, Address, BlockNumber, U256},
};
use alloy_provider::RootProvider;
use alloy_rpc_types::BlockId;
use dotenv::dotenv;
use loom_defi_abi::uniswap2::IUniswapV2Pair;
use loom_node_debug_provider::{AnvilDebugProvider, AnvilDebugProviderFactory};
use std::result::Result;

async fn fetch_pools(
    node_url: String,
    block_number: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    const POOL_ADDRESSES: [Address; 4] = [
        address!("322BBA387c825180ebfB62bD8E6969EBe5b5e52d"), // ITO/WETH pool
        address!("b4e16d0168e52d35cacd2c6185b44281ec28c9dc"), // USDC/WETH pool
        address!("0d4a11d5eeaac28ec3f61d100daf4d40471f1852"), // WETH/USDT pool
        address!("ddd23787a6b80a794d952f5fb036d0b31a8e6aff"), // PEPE/WETH pool
    ];

    let client: AnvilDebugProvider<RootProvider<Ethereum>, RootProvider<Ethereum>, Ethereum> = 
        AnvilDebugProviderFactory::from_node_on_block(node_url, BlockNumber::from(block_number))
        .await
        .unwrap();

    for pool_address in POOL_ADDRESSES {
        let pool_contract = IUniswapV2Pair::new(pool_address, client.clone());
        let contract_reserves = pool_contract
            .getReserves()
            .call()
            .block(BlockId::from(block_number))
            .await?;
        let reserves_0_original = U256::from(contract_reserves.reserve0);
        let reserves_1_original = U256::from(contract_reserves.reserve1);

        println!("Reserve0: {}", reserves_0_original);
        println!("Reserve1: {}", reserves_1_original);
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    let block_number = 22672145u64; // set the latest block number
    let node_url: String = env::var("MAINNET_WS").unwrap(); //add a provider which is supported like tenderly
    let _ = fetch_pools(node_url, block_number).await;
}