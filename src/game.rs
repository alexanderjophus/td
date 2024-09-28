mod camera;
mod placement;
mod wave;

use super::GameState;
use bevy::ecs::system::SystemState;
use bevy::gltf::Gltf;
use bevy::prelude::*;
use bevy_asset_loader::prelude::*;
use bevy_common_assets::ron::RonAssetPlugin;
use camera::CameraPlugin;
use leafwing_input_manager::prelude::*;
use placement::PlacementPlugin;
use std::collections::HashMap;
use std::f32::consts::PI;
use wave::{EnemyDetails, WavePlugin};

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GamePlayState>()
            .add_plugins((
                InputManagerPlugin::<PlayerAction>::default(),
                CameraPlugin,
                PlacementPlugin,
                WavePlugin,
                RonAssetPlugin::<AssetCollections>::new(&["game.ron"]),
            ))
            .init_resource::<Assets<TowerDetails>>()
            .init_resource::<Assets<EnemyDetails>>()
            .init_resource::<ActionState<PlayerAction>>()
            .insert_resource(PlayerAction::default_input_map())
            .add_systems(OnEnter(GameState::Game), setup)
            .add_systems(
                Update,
                (start_wave, end_wave).run_if(in_state(GameState::Game)),
            );
    }
}

// Enum that will be used as a state for the gameplay loop
#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States)]
enum GamePlayState {
    #[default]
    Placement,
    Wave,
}

#[derive(PartialEq, Eq, Clone, Copy, Hash, Debug, Reflect)]
enum PlayerAction {
    MoveCamera,
    MoveCursorPlaceholder,
    ToggleTowerType,
    PlaceTower,
    EndPlacement,
}

impl Actionlike for PlayerAction {
    fn input_control_kind(&self) -> InputControlKind {
        match self {
            PlayerAction::MoveCamera => InputControlKind::DualAxis,
            PlayerAction::MoveCursorPlaceholder => InputControlKind::DualAxis,
            PlayerAction::ToggleTowerType => InputControlKind::Button,
            PlayerAction::PlaceTower => InputControlKind::Button,
            PlayerAction::EndPlacement => InputControlKind::Button,
        }
    }
}

#[derive(Resource, Debug)]
pub struct Resources {
    towers: Vec<AssetId<TowerDetails>>,
    current_tower: usize,
}

impl FromWorld for Resources {
    fn from_world(world: &mut World) -> Self {
        let mut system_state = SystemState::<Res<Assets<TowerDetails>>>::new(world);
        let tower_assets = system_state.get(world);
        let towers = tower_assets.iter().map(|(id, _)| id.clone()).collect();
        Resources {
            towers,
            current_tower: 0,
        }
    }
}

/// Representation of a loaded tower file.
#[derive(Asset, Debug, TypePath, Default)]
pub struct TowerDetails {
    pub name: String,
    pub cost: u32,
    pub range: f32,
    pub damage: u32,
    pub rate_of_fire: f32,
    pub projectile_speed: f32,
    pub model: Handle<Gltf>,
}

#[derive(serde::Deserialize, Debug, Clone)]
enum CustomDynamicAsset {
    Tower {
        name: String,
        cost: u32,
        range: f32,
        damage: u32,
        rate_of_fire: f32,
        projectile_speed: f32,
        model: String,
    },
    Enemy {
        name: String,
        health: u32,
        speed: f32,
        model: String,
    },
}

impl DynamicAsset for CustomDynamicAsset {
    // At this point, the content of your dynamic asset file is done loading.
    // You should return untyped handles to any assets that need to finish loading for your
    // dynamic asset to be ready.
    fn load(&self, asset_server: &AssetServer) -> Vec<UntypedHandle> {
        match self {
            CustomDynamicAsset::Tower { model, .. } => {
                vec![asset_server.load::<Gltf>(model).untyped()]
            }
            CustomDynamicAsset::Enemy { model, .. } => {
                vec![asset_server.load::<Gltf>(model).untyped()]
            }
        }
    }

    // This method is called when all asset handles returned from `load` are done loading.
    // The handles that you return, should also be loaded.
    fn build(&self, world: &mut World) -> Result<DynamicAssetType, anyhow::Error> {
        match self {
            CustomDynamicAsset::Tower {
                name,
                cost,
                range,
                damage,
                rate_of_fire,
                projectile_speed,
                model,
            } => {
                info!(
                    "Building tower: {} with cost: {}, range: {}, damage: {}, rate_of_fire: {}",
                    name, cost, range, damage, rate_of_fire
                );
                let mut gltf_system_state = SystemState::<Res<AssetServer>>::new(world);
                let asset_server = gltf_system_state.get(world);
                let handle: Handle<Gltf> = asset_server.load(model);

                let mut towers_system_state =
                    SystemState::<ResMut<Assets<TowerDetails>>>::new(world);
                let mut towers = towers_system_state.get_mut(world);
                Ok(DynamicAssetType::Single(
                    towers
                        .add(TowerDetails {
                            name: name.clone(),
                            cost: *cost,
                            range: *range,
                            damage: *damage,
                            rate_of_fire: *rate_of_fire,
                            projectile_speed: *projectile_speed,
                            model: handle,
                        })
                        .untyped(),
                ))
            }
            CustomDynamicAsset::Enemy {
                name,
                health,
                speed,
                model,
            } => {
                info!(
                    "Building enemy: {} with health: {}, speed: {}",
                    name, health, speed
                );
                let mut gltf_system_state = SystemState::<Res<AssetServer>>::new(world);
                let asset_server = gltf_system_state.get(world);
                let handle = asset_server.load(model);

                let mut enemies_system_state =
                    SystemState::<ResMut<Assets<EnemyDetails>>>::new(world);
                let mut enemies = enemies_system_state.get_mut(world);
                Ok(DynamicAssetType::Single(
                    enemies
                        .add(EnemyDetails {
                            name: name.clone(),
                            health: *health,
                            speed: *speed,
                            model: handle,
                        })
                        .untyped(),
                ))
            }
        }
    }
}

#[derive(serde::Deserialize, Asset, TypePath)]
pub struct AssetCollections(HashMap<String, CustomDynamicAsset>);

impl DynamicAssetCollection for AssetCollections {
    fn register(&self, dynamic_assets: &mut DynamicAssets) {
        for (key, asset) in self.0.iter() {
            dynamic_assets.register_asset(key, Box::new(asset.clone()));
        }
    }
}

#[derive(AssetCollection, Resource)]
pub struct TowerAssets {
    #[asset(key = "charmander")]
    pub charmander: Handle<TowerDetails>,
    #[asset(key = "gastly")]
    pub gastly: Handle<TowerDetails>,
}

#[derive(AssetCollection, Resource)]
pub struct EnemyAssets {
    #[asset(key = "diglett")]
    pub diglett: Handle<EnemyDetails>,
}

impl PlayerAction {
    /// Define the default bindings to the input
    fn default_input_map() -> InputMap<Self> {
        let mut input_map = InputMap::default();

        // Default gamepad input bindings
        input_map.insert_dual_axis(Self::MoveCamera, GamepadStick::LEFT);
        input_map.insert_dual_axis(Self::MoveCursorPlaceholder, GamepadStick::RIGHT);
        input_map.insert(Self::ToggleTowerType, GamepadButtonType::East);
        input_map.insert(Self::PlaceTower, GamepadButtonType::South);
        input_map.insert(Self::EndPlacement, GamepadButtonType::West);

        // Default kbm input bindings
        input_map.insert_dual_axis(Self::MoveCamera, KeyboardVirtualDPad::WASD);
        input_map.insert_dual_axis(Self::MoveCursorPlaceholder, KeyboardVirtualDPad::ARROW_KEYS);
        input_map.insert(Self::ToggleTowerType, KeyCode::KeyT);
        input_map.insert(Self::PlaceTower, KeyCode::Space);
        input_map.insert(Self::EndPlacement, KeyCode::Enter);

        input_map
    }
}

fn setup(mut commands: Commands) {
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: light_consts::lux::OVERCAST_DAY,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform {
            translation: Vec3::new(0.0, 20.0, 0.0),
            rotation: Quat::from_rotation_x(-PI / 4.),
            ..default()
        },
        ..default()
    });
}

#[derive(Default, Component)]
struct Wave {
    timer: Timer,
}

fn start_wave(
    action_state: Res<ActionState<PlayerAction>>,
    mut next_state: ResMut<NextState<GamePlayState>>,
    mut commands: Commands,
) {
    if action_state.just_pressed(&PlayerAction::EndPlacement) {
        next_state.set(GamePlayState::Wave);
        commands.spawn((Wave {
            timer: Timer::from_seconds(20.0, TimerMode::Once),
        },));
    }
}

fn end_wave(
    mut next_state: ResMut<NextState<GamePlayState>>,
    time: Res<Time>,
    mut query: Query<&mut Wave>,
) {
    for mut wave in query.iter_mut() {
        wave.timer.tick(time.delta());
        if wave.timer.finished() {
            // && enemies are dead
            next_state.set(GamePlayState::Placement);
        }
    }
}
