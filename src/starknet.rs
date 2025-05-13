use std::marker::PhantomData;
use std::str::FromStr;
use std::sync::Arc;

use bevy::prelude::*;
use starknet::{
    accounts::{Account, ExecutionEncoding, SingleOwnerAccount},
    core::types::{Call, Felt, InvokeTransactionResult},
    providers::{JsonRpcClient, Provider, Url, jsonrpc::HttpTransport},
    signers::{LocalWallet, SigningKey},
};
use tokio::sync::mpsc;

use super::tokio::{TokioRuntimeResource, TokioRuntimeState};

/// A structure to hold Starknet connection context information.
#[derive(Clone)]
pub struct StarknetContext {
    /// JSON-RPC client for communicating with a Starknet node
    pub provider: JsonRpcClient<HttpTransport>,
    /// Local wallet used for signing transactions
    pub signer: LocalWallet,
    /// Starknet account address
    pub address: Felt,
}

/// Configuration trait for the Starknet plugin
/// Users of the plugin should implement this trait to provide configuration
pub trait StarknetConfig: Resource + Send + Sync + 'static {
    /// Get the RPC URL for Starknet
    fn rpc_url(&self) -> &str;

    /// Get the private key for the Starknet account
    fn private_key(&self) -> &str;

    /// Get the account address
    fn account_address(&self) -> &str;
}

/// A default implementation of StarknetConfig that reads from environment variables
#[derive(Resource)]
pub struct DefaultStarknetConfig {
    rpc_url: String,
    private_key: String,
    account_address: String,
}

impl DefaultStarknetConfig {
    pub fn new() -> Self {
        Self {
            rpc_url: std::env::var("STARKNET_RPC_URL")
                .unwrap_or_else(|_| "https://starknet-sepolia.blastapi.io/rpc/v0_8".to_string()),
            private_key: std::env::var("STARKNET_PRIVATE_KEY")
                .expect("STARKNET_PRIVATE_KEY environment variable is required"),
            account_address: std::env::var("STARKNET_ACCOUNT_ADDRESS")
                .expect("STARKNET_ACCOUNT_ADDRESS environment variable is required"),
        }
    }

    pub fn with_rpc_url(mut self, rpc_url: impl Into<String>) -> Self {
        self.rpc_url = rpc_url.into();
        self
    }

    pub fn with_private_key(mut self, private_key: impl Into<String>) -> Self {
        self.private_key = private_key.into();
        self
    }

    pub fn with_account_address(mut self, account_address: impl Into<String>) -> Self {
        self.account_address = account_address.into();
        self
    }
}

impl StarknetConfig for DefaultStarknetConfig {
    fn rpc_url(&self) -> &str {
        &self.rpc_url
    }

    fn private_key(&self) -> &str {
        &self.private_key
    }

    fn account_address(&self) -> &str {
        &self.account_address
    }
}

/// Resource for sending commands to the Starknet thread
#[derive(Resource)]
pub struct StarknetChannel {
    tx: mpsc::Sender<StarknetCommand>,
}

impl StarknetChannel {
    /// Send a command to the Starknet thread
    pub fn send(
        &self,
        command: StarknetCommand,
    ) -> Result<(), mpsc::error::TrySendError<StarknetCommand>> {
        self.tx.try_send(command)
    }
}

/// A command to be executed by the Starknet thread
pub enum StarknetCommand {
    /// Execute a transaction with provided calls
    ExecuteTransaction {
        calls: Vec<Call>,
        on_success: Box<dyn FnOnce(InvokeTransactionResult) + Send + 'static>,
        on_error: Box<dyn FnOnce(String) + Send + 'static>,
    },
    /// Custom command with a callback
    Custom {
        handler: Box<
            dyn FnOnce(
                    &SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet>,
                ) -> Box<dyn std::future::Future<Output = ()> + Send + Unpin>
                + Send
                + 'static,
        >,
    },
}

/// States for the Starknet server
#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum StarknetServerState {
    #[default]
    NotReady,
    Ready,
}

/// Starknet plugin for Bevy
/// Generic over the configuration type, allowing users to provide their own configuration
pub struct StarknetPlugin<C: StarknetConfig> {
    _phantom: PhantomData<C>,
}

impl<C: StarknetConfig> Default for StarknetPlugin<C> {
    fn default() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<C: StarknetConfig> Plugin for StarknetPlugin<C> {
    fn build(&self, app: &mut App) {
        app.init_state::<StarknetServerState>();
        app.add_systems(
            OnEnter(TokioRuntimeState::Ready),
            spawn_starknet_thread::<C>,
        );
    }
}

/// Spawn the Starknet thread for processing commands
fn spawn_starknet_thread<C: StarknetConfig>(
    mut commands: Commands,
    rt: Res<TokioRuntimeResource>,
    mut next_state: ResMut<NextState<StarknetServerState>>,
    config: Res<C>,
) {
    let (tx, mut rx) = mpsc::channel::<StarknetCommand>(64);

    let rpc_url = config.rpc_url().to_string();
    let private_key = config.private_key().to_string();
    let account_address = config.account_address().to_string();

    let _ = rt.0.spawn(async move {
        let url = match Url::parse(&rpc_url) {
            Ok(url) => url,
            Err(e) => {
                error!("Invalid RPC URL: {:?}", e);
                return;
            }
        };

        // Create provider - JsonRpcClient::new returns a JsonRpcClient directly, not a Result
        let provider = JsonRpcClient::new(HttpTransport::new(url));

        let signer = LocalWallet::from(SigningKey::from_secret_scalar(
            Felt::from_str(&private_key).expect("Invalid private key"),
        ));

        let address = match Felt::from_str(&account_address) {
            Ok(address) => address,
            Err(e) => {
                error!("Invalid account address: {:?}", e);
                return;
            }
        };

        let chain_id = match provider.chain_id().await {
            Ok(id) => id,
            Err(e) => {
                error!("Failed to get chain ID: {:?}", e);
                return;
            }
        };

        // Create the account
        let account = SingleOwnerAccount::new(
            provider.clone(),
            signer.clone(),
            address,
            chain_id,
            ExecutionEncoding::New,
        );

        // Create a lock to prevent concurrent transactions
        let processing_lock = Arc::new(tokio::sync::Mutex::new(()));

        info!("Started STARKNET TX PROCESSING SERVER...");

        // Process incoming commands
        while let Some(command) = rx.recv().await {
            // Try to acquire the lock - if already locked, skip this command
            if let Ok(_lock) = processing_lock.try_lock() {
                match command {
                    StarknetCommand::ExecuteTransaction {
                        calls,
                        on_success,
                        on_error,
                    } => match account.execute_v3(calls).send().await {
                        Ok(result) => {
                            on_success(result);
                        }
                        Err(e) => {
                            let error_msg = format!("Transaction execution failed: {:?}", e);
                            error!("{}", error_msg);
                            on_error(error_msg);
                        }
                    },
                    StarknetCommand::Custom { handler } => {
                        let future = handler(&account);
                        future.await;
                    }
                }
            } else {
                info!("Already processing a transaction, skipping new request");
            }
        }
    });

    commands.insert_resource(StarknetChannel { tx });
    next_state.set(StarknetServerState::Ready);
}

/// Execute a Starknet transaction
pub fn execute_starknet_transaction(
    channel: &StarknetChannel,
    calls: Vec<Call>,
    on_success: impl FnOnce(InvokeTransactionResult) + Send + 'static,
    on_error: impl FnOnce(String) + Send + 'static,
) -> Result<(), mpsc::error::TrySendError<StarknetCommand>> {
    channel.send(StarknetCommand::ExecuteTransaction {
        calls,
        on_success: Box::new(on_success),
        on_error: Box::new(on_error),
    })
}

/// Creates a StarknetContext from environment variables.
///
/// # Returns
///
/// * `Result<StarknetContext, Box<dyn std::error::Error>>` - Structure containing provider, signer, and address or an error
///
/// # Environment Variables
///
/// * `STARKNET_RPC_URL` - URL of the Starknet JSON-RPC endpoint
/// * `STARKNET_PRIVATE_KEY` - Private key for the Starknet account
/// * `STARKNET_ACCOUNT_ADDRESS` - Address of the Starknet account
///
/// # Example
///
/// ```no_run
/// # use starknet_lib::starknet_call_context_from_env;
/// let context = starknet_call_context_from_env().expect("Failed to create context");
/// ```
pub fn starknet_call_context_from_env() -> Result<StarknetContext, Box<dyn std::error::Error>> {
    let rpc_url = std::env::var("STARKNET_RPC_URL")
        .map_err(|_| "Cannot find STARKNET_RPC_URL env variable")?;

    let private_key = std::env::var("STARKNET_PRIVATE_KEY")
        .map_err(|_| "Cannot find STARKNET_PRIVATE_KEY env variable")?;

    let account_address = std::env::var("STARKNET_ACCOUNT_ADDRESS")
        .map_err(|_| "Cannot find STARKNET_ACCOUNT_ADDRESS env variable")?;

    let provider = JsonRpcClient::new(HttpTransport::new(Url::parse(&rpc_url)?));

    let signer = LocalWallet::from(SigningKey::from_secret_scalar(Felt::from_str(
        &private_key,
    )?));

    let address = Felt::from_str(&account_address)?;

    Ok(StarknetContext {
        provider,
        signer,
        address,
    })
}

/// Creates a StarknetContext from provided parameters.
///
/// # Arguments
///
/// * `rpc_url` - URL of the Starknet JSON-RPC endpoint
/// * `private_key` - Private key for the Starknet account
/// * `account_address` - Address of the Starknet account
///
/// # Returns
///
/// * `Result<StarknetContext, Box<dyn std::error::Error>>` - Structure containing provider, signer, and address or an error
///
/// # Example
///
/// ```no_run
/// # use bevy_dojo::starknet_account_context;
/// let context = starknet_call_context(
///     "https://starknet-sepolia.blastapi.io/rpc/v0_8",
///     "0x1234...",
///     "0x5678..."
/// ).expect("Failed to create context");
/// ```
pub fn starknet_account_context(
    rpc_url: &str,
    private_key: &str,
    account_address: &str,
) -> Result<StarknetContext, Box<dyn std::error::Error>> {
    let provider = JsonRpcClient::new(HttpTransport::new(Url::parse(rpc_url)?));

    let signer = LocalWallet::from(SigningKey::from_secret_scalar(Felt::from_str(private_key)?));

    let address = Felt::from_str(account_address)?;

    Ok(StarknetContext {
        provider,
        signer,
        address,
    })
}

/// Creates a SingleOwnerAccount from the provided components.
///
/// # Arguments
///
/// * `provider` - JSON-RPC client for Starknet node communication
/// * `signer` - Wallet for transaction signing
/// * `address` - Account address
/// * `chain_id` - Starknet chain ID (e.g., chain_id::SEPOLIA)
///
/// # Returns
///
/// * `SingleOwnerAccount` - The initialized Starknet account
///
/// # Example
///
/// ```no_run
/// # use bevy_dojo::{starknet_account, StarknetContext};
/// # use starknet::core::chain_id;
/// # fn example(context: StarknetContext) {
/// let account = starknet_account(
///     context.provider,
///     context.signer,
///     context.address,
///     chain_id::SEPOLIA
/// );
/// # }
/// ```
pub fn starknet_account(
    provider: JsonRpcClient<HttpTransport>,
    signer: LocalWallet,
    address: Felt,
    chain_id: Felt,
) -> SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet> {
    SingleOwnerAccount::new(provider, signer, address, chain_id, ExecutionEncoding::New)
}
