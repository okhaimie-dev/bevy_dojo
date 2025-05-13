use bevy::prelude::*;
use tokio::runtime::Runtime;

pub struct TokioPlugin;
impl Plugin for TokioPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<TokioRuntimeState>();
        app.add_systems(
            Update,
            setup_tokio_runtime.run_if(in_state(TokioRuntimeState::NotReady)),
        );
    }
}

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq, States)]
pub enum TokioRuntimeState {
    #[default]
    NotReady,
    Ready,
}

#[derive(Resource)]
pub struct TokioRuntimeResource(pub Runtime);

fn setup_tokio_runtime(
    mut commands: Commands,
    mut next_state: ResMut<NextState<TokioRuntimeState>>,
) {
    let runtime = tokio::runtime::Runtime::new();
    match runtime {
        Ok(rt) => {
            commands.insert_resource(TokioRuntimeResource(rt));
            next_state.set(TokioRuntimeState::Ready);
            println!("Tokio is READY!")
        }
        Err(e) => println!("{e:?}"),
    }
}
