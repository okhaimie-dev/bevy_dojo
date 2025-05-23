use bevy::prelude::*;
use tokio::runtime::Runtime;

/// Plugin that initializes the Tokio runtime
///
/// This plugin is automatically added when you use the `BevyDojoPlugin`.
/// It creates a multi-threaded Tokio runtime for executing async tasks.
pub struct TokioPlugin;
impl Plugin for TokioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TokioRuntime>();
    }
}

/// Resource that holds the Tokio runtime
///
/// This resource provides access to the Tokio runtime for executing async tasks.
/// It is automatically added when you use the `TokioPlugin` or `BevyDojoPlugin`.
///
/// # Example
///
/// ```no_run
/// fn my_system(runtime: Res<TokioRuntime>) {
///     // Spawn an async task
///     runtime.runtime.spawn(async {
///         // Do something async
///     });
/// }
/// ```
#[derive(Resource)]
pub struct TokioRuntime {
    pub runtime: Runtime,
}

impl Default for TokioRuntime {
    fn default() -> Self {
        Self {
            runtime: Runtime::new().expect("Failed to create Tokio runtime"),
        }
    }
}
