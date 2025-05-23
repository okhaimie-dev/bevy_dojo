use bevy::prelude::*;

use std::collections::VecDeque;
use std::str::FromStr;
use std::sync::Arc;

use crate::tokio::TokioRuntime;
use starknet::accounts::single_owner::SignError;
use starknet::signers::local_wallet::SignError as LocalWalletSignError;
use starknet::{
    accounts::{Account, AccountError, ExecutionEncoding, SingleOwnerAccount},
    core::types::{Call, Felt, InvokeTransactionResult},
    providers::{AnyProvider, JsonRpcClient, Provider, Url, jsonrpc::HttpTransport},
    signers::{LocalWallet, SigningKey},
};

use tokio::task::JoinHandle;

/// Resource to store Starknet connection state
///
/// This resource manages the connection to Starknet and tracks pending transactions.
/// It is automatically added to the app when using the `BevyDojoPlugin`.
///
/// # Usage
///
/// Access this resource in your systems to check connection status or interact
/// with the Starknet blockchain:
///
/// ```no_run
/// fn my_system(
///     sn: Res<StarknetConnection>,
///     runtime: Res<TokioRuntime>,
///     config: Res<DefaultStarknetConfig>,
/// ) {
///     if !sn.is_connected() {
///         // Initialize connection if needed
///     }
/// }
/// ```
#[derive(Resource, Default)]
pub struct StarknetConnection {
    connecting_task: Option<JoinHandle<Arc<SingleOwnerAccount<AnyProvider, LocalWallet>>>>,
    account: Option<Arc<SingleOwnerAccount<AnyProvider, LocalWallet>>>,
    pending_txs: VecDeque<
        JoinHandle<Result<InvokeTransactionResult, AccountError<SignError<LocalWalletSignError>>>>,
    >,
}

impl StarknetConnection {
    /// Returns true if the connection is established
    pub fn is_connected(&self) -> bool {
        self.account.is_some()
    }

    /// Returns true if currently trying to establish a connection
    pub fn is_connecting(&self) -> bool {
        self.connecting_task.is_some()
    }

    /// Returns the number of pending transactions
    pub fn pending_tx_count(&self) -> usize {
        self.pending_txs.len()
    }
}

/// Default configuration for Starknet integration
///
/// This resource provides configuration for connecting to Starknet.
/// By default, it reads values from environment variables:
/// - `STARKNET_RPC_URL`: URL of your Starknet RPC provider
/// - `STARKNET_ACCOUNT_ADDRESS`: Your Starknet account address (as a hex string)
/// - `STARKNET_PRIVATE_KEY`: Your private key (as a hex string)
///
/// # Custom Configuration
///
/// You can replace this resource with your own configuration:
///
/// ```no_run
/// fn setup(mut commands: Commands) {
///     commands.insert_resource(DefaultStarknetConfig {
///         rpc_url: "https://starknet-mainnet.infura.io/v3/YOUR_API_KEY".to_string(),
///         account_address: "0x123...".to_string(),
///         private_key: "0x456...".to_string(),
///     });
/// }
/// ```
#[derive(Resource, Clone)]
pub struct DefaultStarknetConfig {
    pub rpc_url: String,
    pub account_address: String,
    pub private_key: String,
}

impl Default for DefaultStarknetConfig {
    fn default() -> Self {
        Self {
            rpc_url: std::env::var("STARKNET_RPC_URL").unwrap_or_default(),
            account_address: std::env::var("STARKNET_ACCOUNT_ADDRESS").unwrap_or_default(),
            private_key: std::env::var("STARKNET_PRIVATE_KEY").unwrap_or_default(),
        }
    }
}

/// Initialize a connection to Starknet
///
/// This function spawns an async task to connect to Starknet using the provided configuration.
/// The connection status can be monitored through the `StarknetConnection` resource.
///
/// # Arguments
///
/// * `runtime` - The Tokio runtime resource
/// * `config` - The Starknet configuration resource
/// * `sn` - The Starknet connection resource
///
/// # Example
///
/// ```no_run
/// fn my_system(
///     runtime: Res<TokioRuntime>,
///     config: Res<DefaultStarknetConfig>,
///     mut sn: ResMut<StarknetConnection>,
/// ) {
///     // Initialize connection only if not already connected or connecting
///     if !sn.is_connected() && !sn.is_connecting() {
///         init_starknet_connection(runtime, config, sn);
///     }
/// }
/// ```
pub fn init_starknet_connection(
    runtime: Res<TokioRuntime>,
    config: Res<DefaultStarknetConfig>,
    mut sn: ResMut<StarknetConnection>,
) {
    if sn.connecting_task.is_none() && sn.account.is_none() {
        let config_clone = config.clone();
        let handle = runtime
            .runtime
            .spawn(async move { connect_to_starknet(config_clone).await });
        sn.connecting_task = Some(handle);
        info!("Connecting to Starknet...");
    }
}

/// Execute a Starknet transaction
///
/// This function adds a transaction to a queue to be processed in the background.
/// The result will be automatically checked by the `check_sn_task` system, which
/// is registered by the `BevyDojoPlugin`.
///
/// # Arguments
///
/// * `runtime` - The Tokio runtime resource
/// * `sn` - The Starknet connection resource
/// * `calls` - A vector of Starknet calls to execute
///
/// # Returns
///
/// * `true` if the transaction was queued successfully
/// * `false` if there's no active Starknet connection
///
/// # Example
///
/// ```no_run
/// use starknet::core::types::{Call, Felt};
/// use std::str::FromStr;
///
/// fn execute_increment(
///     runtime: Res<TokioRuntime>,
///     mut sn: ResMut<StarknetConnection>,
/// ) {
///     let contract_address = Felt::from_str("0x123...").unwrap();
///     let selector = Felt::from_str("0x362398bec32bc0ebb411203221a35a0301193a96f317ebe5e40be9f60d15320").unwrap(); // "increment"
///
///     let calls = vec![
///         Call {
///             to: contract_address,
///             selector,
///             calldata: vec![],
///         },
///     ];
///
///     if execute_transaction(runtime, sn, calls) {
///         println!("Transaction submitted!");
///     } else {
///         println!("Not connected to Starknet!");
///     }
/// }
/// ```
pub fn execute_transaction(
    runtime: Res<TokioRuntime>,
    mut sn: ResMut<StarknetConnection>,
    calls: Vec<Call>,
) -> bool {
    if let Some(account) = sn.account.clone() {
        let task = runtime.runtime.spawn(async move {
            // Create the transaction inside the async block where we own the account
            let tx = account.execute_v3(calls);
            tx.send().await
        });
        sn.pending_txs.push_back(task);
        true
    } else {
        false
    }
}

/// System that checks the status of Starknet tasks
///
/// This system:
/// 1. Checks if a connection task has completed and updates the connection state
/// 2. Checks pending transactions and logs their completion
///
/// It is automatically registered by the `BevyDojoPlugin` and should run every frame.
///
/// # Arguments
///
/// * `runtime` - The Tokio runtime resource
/// * `sn` - The Starknet connection resource
pub fn check_sn_task(runtime: Res<TokioRuntime>, mut sn: ResMut<StarknetConnection>) {
    // Check connection task
    if let Some(task) = &mut sn.connecting_task {
        if let Ok(account) = runtime.runtime.block_on(async { task.await }) {
            info!("Connected to Starknet!");
            sn.account = Some(account);
            sn.connecting_task = None;
        }
    }

    // Check pending transactions
    if !sn.pending_txs.is_empty() && sn.account.is_some() {
        if let Some(task) = sn.pending_txs.pop_front() {
            if let Ok(Ok(result)) = runtime.runtime.block_on(async { task.await }) {
                info!("Transaction completed: {:#x}", result.transaction_hash);
            }
        }
    }
}

/// Connect to Starknet using the provided configuration
///
/// This is an async function that establishes a connection to Starknet.
/// It is typically called by `init_starknet_connection` rather than directly.
///
/// # Arguments
///
/// * `config` - The Starknet configuration
///
/// # Returns
///
/// An Arc-wrapped SingleOwnerAccount that can be used to interact with Starknet
pub async fn connect_to_starknet(
    config: DefaultStarknetConfig,
) -> Arc<SingleOwnerAccount<AnyProvider, LocalWallet>> {
    let provider = AnyProvider::JsonRpcHttp(JsonRpcClient::new(HttpTransport::new(
        Url::parse(&config.rpc_url).expect("Invalid RPC URL"),
    )));
    let account_addr = Felt::from_str(&config.account_address).expect("Invalid account address");
    let chain_id = provider.chain_id().await.unwrap();
    let signer = LocalWallet::from(SigningKey::from_secret_scalar(
        Felt::from_str(&config.private_key).expect("Invalid private key"),
    ));

    Arc::new(SingleOwnerAccount::new(
        provider,
        signer,
        account_addr,
        chain_id,
        ExecutionEncoding::New,
    ))
}
