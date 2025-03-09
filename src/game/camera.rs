use bevy::prelude::*;
use leafwing_input_manager::{
    plugin::InputManagerPlugin, prelude::*, Actionlike, InputControlKind,
};

use crate::GameState;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(InputManagerPlugin::<CameraAction>::default())
            .init_resource::<ActionState<CameraAction>>()
            .insert_resource(CameraAction::default_input_map())
            .add_systems(OnEnter(GameState::Game), setup)
            .add_systems(Update, control_camera.run_if(in_state(GameState::Game)));
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Hash, Debug, Reflect)]
enum CameraAction {
    MoveCamera,
}

impl Actionlike for CameraAction {
    fn input_control_kind(&self) -> InputControlKind {
        match self {
            CameraAction::MoveCamera => InputControlKind::DualAxis,
        }
    }
}

impl CameraAction {
    /// Define the default bindings to the input
    fn default_input_map() -> InputMap<Self> {
        let mut input_map = InputMap::default();

        // Default gamepad input bindings
        input_map.insert_dual_axis(Self::MoveCamera, GamepadStick::LEFT);

        // // Default kbm input bindings
        input_map.insert_dual_axis(Self::MoveCamera, VirtualDPad::wasd());

        input_map
    }
}

#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct FollowCam;

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 2.0, 5.0).looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
        FollowCam,
    ));

    // spawn 2D overlay
    commands.spawn((
        Camera2d,
        Camera {
            order: 1,
            clear_color: ClearColorConfig::None,
            ..Default::default()
        },
    ));
}

fn control_camera(
    time: Res<Time>,
    action_state: Res<ActionState<CameraAction>>,
    mut query: Query<&mut Transform, With<FollowCam>>,
) {
    let mut player_transform = query.single_mut();
    let move_delta = time.delta_secs()
        * action_state
            .clamped_axis_pair(&CameraAction::MoveCamera)
            .xy();
    player_transform.translation += Vec3::new(move_delta.x, 0.0, -move_delta.y);
}
