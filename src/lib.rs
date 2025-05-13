//! Bevy Dojo: A Bevy plugin for Starknet integration
//!
//! This crate provides a plugin for integrating Starknet blockchain functionality
//! with the Bevy game engine. It enables executing Starknet transactions from Bevy
//! systems in a non-blocking way.
//!
//! # Example
//!
//! ```no_run
//! use bevy::prelude::*;
//! use bevy_dojo::prelude::*;
//!
//! fn main() {
//!     App::new()
//!         .add_plugins(DefaultPlugins)
//!         .add_plugins(TokioPlugin)
//!         .insert_resource(DefaultStarknetConfig::new())
//!         .add_plugins(StarknetPlugin::<DefaultStarknetConfig>::default())
//!         .add_systems(Update, execute_starknet_tx_system)
//!         .run();
//! }
//!
//! fn execute_starknet_tx_system(
//!     keyboard_input: Res<Input<KeyCode>>,
//!     starknet_channel: Option<Res<StarknetChannel>>,
//! ) {
//!     if keyboard_input.just_pressed(KeyCode::Space) {
//!         if let Some(channel) = starknet_channel {
//!             // Example of sending a custom transaction
//!             // ...
//!         }
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
        DefaultStarknetConfig, StarknetChannel, StarknetCommand, StarknetConfig, StarknetPlugin,
        StarknetServerState, execute_starknet_transaction, starknet_account_context,
        starknet_call_context_from_env,
    };
    pub use crate::tokio::{TokioPlugin, TokioRuntimeState};

    // Re-export commonly used Starknet types
    pub use starknet::{
        accounts::{Account, SingleOwnerAccount},
        core::{
            types::{Call, Felt, InvokeTransactionResult},
            utils::get_selector_from_name,
        },
    };
}

// Export the main plugin as a convenience for users
/// Starknet integration plugin with default configuration
pub struct BevyDojoPlugin;

impl Plugin for BevyDojoPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(tokio::TokioPlugin);
        app.insert_resource(starknet::DefaultStarknetConfig::new());
        app.add_plugins(starknet::StarknetPlugin::<starknet::DefaultStarknetConfig>::default());
    }
}
