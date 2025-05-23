//! # Bevy Dojo: A Bevy plugin for Starknet integration
//!
//! `bevy_dojo` provides a plugin for integrating Starknet blockchain functionality
//! with the Bevy game engine. It enables executing Starknet transactions from Bevy
//! systems in a non-blocking way, making it ideal for games that want to interact
//! with smart contracts on Starknet.
//!
//! ## Features
//!
//! - Non-blocking Starknet connection management
//! - Transaction execution with automatic status monitoring
//! - Environment variable or explicit configuration options
//! - Seamless integration with Bevy's ECS
//!
//! ## Setup
//!
//! Add the plugin to your Bevy app:
//!
//! ```rust
//! use bevy::prelude::*;
//! use bevy_dojo::prelude::*;
//!
//! fn main() {
//!     App::new()
//!         .add_plugins(DefaultPlugins)
//!         .add_plugins(BevyDojoPlugin)
//!         .add_systems(Update, keyboard_control)
//!         .run();
//! }
//! ```
//!
//! ## Environment Variables
//!
//! The plugin uses the following environment variables by default:
//!
//! - `STARKNET_RPC_URL`: URL of your Starknet RPC provider
//! - `STARKNET_ACCOUNT_ADDRESS`: Your Starknet account address (as a hex string)
//! - `STARKNET_PRIVATE_KEY`: Your private key (as a hex string)
//!
//! Alternatively, you can provide these values explicitly by replacing the
//! `DefaultStarknetConfig` resource.
//!
//! ## Example: Keyboard-controlled Connection and Transactions
//!
//! ```no_run
//! use bevy::prelude::*;
//! use bevy_dojo::prelude::*;
//! use starknet::core::types::{Call, Felt};
//! use std::str::FromStr;
//!
//! fn keyboard_control(
//!     keys: Res<Input<KeyCode>>,
//!     runtime: Res<TokioRuntime>,
//!     config: Res<DefaultStarknetConfig>,
//!     mut sn: ResMut<StarknetConnection>,
//! ) {
//!     // Connect to Starknet when the user presses C
//!     if keys.just_pressed(KeyCode::C) {
//!         init_starknet_connection(runtime, config, sn);
//!     }
//!
//!     // Execute a transaction when the user presses T
//!     if keys.just_pressed(KeyCode::T) {
//!         let calls = vec![
//!             Call {
//!                 to: Felt::from_str("0x123456...").unwrap(),  // Contract address
//!                 selector: Felt::from_str("0x987654...").unwrap(),  // Function selector
//!                 calldata: vec![],  // Function arguments
//!             },
//!         ];
//!
//!         execute_transaction(runtime, sn, calls);
//!     }
//! }
//! ```
//!
//! ## Connection Status
//!
//! You can check the connection status by examining the `StarknetConnection` resource:
//!
//! ```no_run
//! fn display_connection_status(sn: Res<StarknetConnection>) {
//!     if sn.is_connected() {
//!         println!("Connected to Starknet");
//!     } else if sn.is_connecting() {
//!         println!("Connecting to Starknet...");
//!     } else {
//!         println!("Not connected to Starknet");
//!     }
//! }
//! ```

// Re-export modules
pub mod starknet;
pub mod tokio;

// Import and re-export main types for convenience
use bevy::prelude::*;

// Main prelude module that users can import
pub mod prelude {
    pub use crate::starknet::{
        DefaultStarknetConfig, StarknetConnection, check_sn_task, connect_to_starknet,
        init_starknet_connection,
    };
    pub use crate::tokio::{TokioPlugin, TokioRuntime};

    // Re-export commonly used Starknet types
    pub use starknet::{
        accounts::{Account, SingleOwnerAccount},
        core::{
            types::{Call, Felt, InvokeTransactionResult},
            utils::get_selector_from_name,
        },
    };
}

/// Starknet integration plugin with default configuration
///
/// This plugin initializes all resources needed for Starknet integration:
/// - Adds the `TokioPlugin` to create a Tokio runtime
/// - Initializes the `StarknetConnection` resource
/// - Initializes the `DefaultStarknetConfig` resource
/// - Registers the `check_sn_task` system to monitor async tasks
///
/// # Example
///
/// ```no_run
/// use bevy::prelude::*;
/// use bevy_dojo::prelude::*;
///
/// fn main() {
///     App::new()
///         .add_plugins(DefaultPlugins)
///         .add_plugins(BevyDojoPlugin)
///         .run();
/// }
/// ```
pub struct BevyDojoPlugin;

impl Plugin for BevyDojoPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(tokio::TokioPlugin)
            .init_resource::<starknet::StarknetConnection>()
            .init_resource::<starknet::DefaultStarknetConfig>()
            .add_systems(Update, starknet::check_sn_task);
    }
}
