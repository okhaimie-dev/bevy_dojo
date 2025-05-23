# Bevy Dojo

A Bevy plugin for Starknet blockchain integration.

## Features

- Non-blocking Starknet connection management
- Transaction execution with automatic status monitoring
- Environment variable or explicit configuration options
- Seamless integration with Bevy's ECS

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
bevy_dojo = "0.0.2"
```

## Usage

### Basic Setup

```rs
use bevy::prelude::*;
use bevy_dojo::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(BevyDojoPlugin)
        .add_systems(Update, keyboard_control)
        .run();
}

fn keyboard_control(
    keys: Res<Input<KeyCode>>,
    runtime: Res<TokioRuntime>,
    config: Res<DefaultStarknetConfig>,
    mut sn: ResMut<StarknetConnection>,
) {
    // Connect to Starknet when the user presses C
    if keys.just_pressed(KeyCode::C) {
        init_starknet_connection(runtime, config, sn);
    }

    // Execute a transaction when the user presses T
    if keys.just_pressed(KeyCode::T) {
        // Example: Call a contract function
        let calls = vec![
            Call {
                to: Felt::from_str("0x123456...").unwrap(),  // Contract address
                selector: Felt::from_str("0x987654...").unwrap(),  // Function selector
                calldata: vec![],  // Function arguments
            },
        ];

        execute_transaction(runtime, sn, calls);
    }
}
```

## Configuration

Configuration

By default, the plugin reads configuration from environment variables:

- `STARKNET_RPC_URL`: URL of your Starknet RPC provider
- `STARKNET_ACCOUNT_ADDRESS`: Your Starknet account address (as a hex string)
- `STARKNET_PRIVATE_KEY`: Your private key (as a hex string)

You can also provide explicit configuration:

```rs
fn setup(mut commands: Commands) {
    commands.insert_resource(DefaultStarknetConfig {
        rpc_url: "https://starknet-mainnet.infura.io/v3/YOUR_API_KEY".to_string(),
        account_address: "0x123...".to_string(),
        private_key: "0x456...".to_string(),
    });
}
```

## License

This crate is licensed under MIT License
