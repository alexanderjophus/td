mod camera;
mod dice_physics;
mod economy;
mod placement;
mod roll;
mod wave;

use super::GameState;

use avian3d::prelude::*;
use bevy::math::vec2;
use bevy::prelude::*;
use bevy::utils::HashMap;
use bevy::{ecs::system::SystemState, gltf::Gltf, render::primitives::Aabb};
use bevy_asset_loader::prelude::*;
use bevy_common_assets::ron::RonAssetPlugin;
use camera::CameraPlugin;
use economy::EconomyPlugin;
use placement::PlacementPlugin;
use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};
use roll::RollPlugin;
use std::f32::consts::PI;
use vleue_navigator::prelude::*;
use wave::WavePlugin;

const SNAP_OFFSET: f32 = 0.5;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GamePlayState>()
            .add_plugins((
                CameraPlugin,
                EconomyPlugin,
                PlacementPlugin,
                RollPlugin,
                WavePlugin,
                PhysicsPlugins::default(),
                #[cfg(feature = "debug")]
                PhysicsDebugPlugin::default(),
                RonAssetPlugin::<AssetCollections>::new(&["game.ron"]),
                VleueNavigatorPlugin,
                NavmeshUpdaterPlugin::<Aabb, Obstacle>::default(),
            ))
            .init_resource::<Assets<TowerDetails>>()
            .init_resource::<Assets<EnemyDetails>>()
            .init_resource::<GameResources>()
            .register_type::<GameResources>()
            .register_type::<uuid::Uuid>()
            .add_event::<DiePurchaseEvent>()
            .add_event::<DieRolledEvent>()
            .add_event::<DieRollResultEvent>()
            .add_systems(OnEnter(GameState::Game), setup)
            .add_systems(Update, (die_purchased).run_if(in_state(GameState::Game)));
    }
}

// Enum that will be used as a state for the gameplay loop
#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States)]
enum GamePlayState {
    #[default]
    Economy,
    Rolling,
    Placement,
    Wave,
}

#[derive(Component, Debug)]
struct Obstacle;

#[derive(AssetCollection, Resource)]
#[allow(dead_code)]
pub struct AllAssets {
    #[asset(key = "towers", collection(typed))]
    pub towers: Vec<Handle<TowerDetails>>,
    #[asset(key = "enemies", collection(typed))]
    pub enemies: Vec<Handle<EnemyDetails>>,
}

#[derive(Resource, Debug, Clone, PartialEq, Reflect)]
#[reflect(Resource)]
pub struct GameResources {
    money: usize,
    dice: Vec<Die>,
    highlighted_die: usize,
    towers: Vec<AssetId<TowerDetails>>,
    highlighted_tower: usize,
}

impl Default for GameResources {
    fn default() -> Self {
        GameResources {
            money: 50,
            dice: Vec::new(),
            highlighted_die: 0,
            towers: Vec::new(),
            highlighted_tower: 0,
        }
    }
}

/// Representation of a loaded tower file.
#[derive(Asset, Resource, Component, Debug, PartialEq, Clone, TypePath)]
pub struct TowerDetails {
    pub name: String,
    pub element_type: BaseElementType,
    pub model: Handle<Gltf>,
}

/// Representation of a loaded enemy file.
#[derive(Asset, Debug, TypePath)]
pub struct EnemyDetails {
    pub name: String,
    pub health: u32,
    pub speed: f32,
    pub model: Handle<Gltf>,
}

#[derive(serde::Deserialize, Debug, Clone)]
enum CustomDynamicAsset {
    Towers(Vec<TowerDetailsRon>),
    Enemies(Vec<EnemyDetailsRon>),
}

impl DynamicAsset for CustomDynamicAsset {
    fn load(&self, asset_server: &AssetServer) -> Vec<UntypedHandle> {
        match self {
            CustomDynamicAsset::Towers(towers) => towers
                .iter()
                .map(|tower| asset_server.load::<Gltf>(tower.model.clone()).untyped())
                .collect(),
            CustomDynamicAsset::Enemies(enemies) => enemies
                .iter()
                .map(|enemy| asset_server.load::<Gltf>(enemy.model.clone()).untyped())
                .collect(),
        }
    }

    fn build(&self, world: &mut World) -> Result<DynamicAssetType, anyhow::Error> {
        match self {
            CustomDynamicAsset::Towers(towers) => {
                let mut towers_collection = vec![];
                for tower in towers {
                    let model = world
                        .get_resource::<AssetServer>()
                        .unwrap()
                        .load(tower.model.clone());
                    let mut tower_details =
                        SystemState::<ResMut<Assets<TowerDetails>>>::new(world).get_mut(world);
                    let handle = tower_details.add(TowerDetails {
                        name: tower.name.clone(),
                        element_type: tower.element_type,
                        model: model.clone(),
                    });
                    towers_collection.push(handle.untyped());
                    info!("Built tower: {}", tower.name);
                }
                Ok(DynamicAssetType::Collection(towers_collection))
            }
            CustomDynamicAsset::Enemies(enemies) => {
                let mut enemies_collection = vec![];
                for enemy in enemies {
                    let model = world
                        .get_resource::<AssetServer>()
                        .unwrap()
                        .load(enemy.model.clone());
                    let mut assets = world.get_resource_mut::<Assets<EnemyDetails>>().unwrap();
                    let handle = assets.add(EnemyDetails {
                        name: enemy.name.clone(),
                        health: enemy.health,
                        speed: enemy.speed,
                        model: model.clone(),
                    });
                    enemies_collection.push(handle.untyped());
                    info!("Built enemy: {}", enemy.name);
                }
                Ok(DynamicAssetType::Collection(enemies_collection))
            }
        }
    }
}

#[derive(serde::Deserialize, Asset, Debug, TypePath, Clone)]
pub struct TowerDetailsRon {
    pub name: String,
    pub element_type: BaseElementType,
    pub model: String,
}

#[derive(serde::Deserialize, Asset, Debug, TypePath, Clone)]
pub struct EnemyDetailsRon {
    pub name: String,
    pub health: u32,
    pub speed: f32,
    pub model: String,
}

#[derive(AssetCollection, Resource)]
pub struct GltfAssets {
    #[asset(path = "models/dungeon.glb#Scene0")]
    pub dungeon: Handle<Scene>,
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

#[derive(Default, Component)]
struct Goal;

#[derive(Resource, Debug, Clone, PartialEq, Copy, Reflect)]
#[reflect(Resource)]
struct DieFace {
    primary_type: BaseElementType,
    rarity: Rarity,
}

impl DieFace {
    pub fn new(primary_type: BaseElementType, rarity: Rarity) -> Self {
        DieFace {
            primary_type,
            rarity,
        }
    }

    pub fn generate(base_element: BaseElementType, base_rarity: Rarity) -> Self {
        let mut rng = thread_rng();

        // chance to change element type
        let final_element = if rng.gen_bool(0.25) {
            let elements = [
                BaseElementType::Earth,
                BaseElementType::Fire,
                BaseElementType::Water,
                BaseElementType::Wind,
            ];
            // Keep rolling until we get a different element
            loop {
                let new_element = *elements.choose(&mut rng).unwrap();
                if new_element != base_element {
                    break new_element;
                }
            }
        } else {
            base_element
        };

        // chance to upgrade rarity
        let final_rarity = if rng.gen_bool(0.05) {
            match base_rarity {
                Rarity::Common => Rarity::Uncommon,
                Rarity::Uncommon => Rarity::Rare,
                Rarity::Rare => Rarity::Epic,
                Rarity::Epic => Rarity::Unique,
                Rarity::Unique => Rarity::Unique,
            }
        } else {
            base_rarity
        };

        DieFace {
            primary_type: final_element,
            rarity: final_rarity,
        }
    }
}

#[derive(Resource, serde::Deserialize, Default, Debug, Clone, Copy, PartialEq, Reflect)]
#[reflect(Resource)]
pub enum BaseElementType {
    #[default]
    None, // No element
    Fire,  // Heat and destruction
    Water, // Flow and adaptability
    Earth, // Stability and strength
    Wind,  // Movement and agility
}

impl std::fmt::Display for BaseElementType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BaseElementType::None => write!(f, "None"),
            BaseElementType::Fire => write!(f, "Fire"),
            BaseElementType::Water => write!(f, "Water"),
            BaseElementType::Earth => write!(f, "Earth"),
            BaseElementType::Wind => write!(f, "Wind"),
        }
    }
}

#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Reflect)]
#[reflect(Resource)]
enum Rarity {
    #[default]
    Common,
    Uncommon,
    Rare,
    Epic,
    Unique,
}

impl std::fmt::Display for Rarity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Rarity::Common => write!(f, "Common"),
            Rarity::Uncommon => write!(f, "Uncommon"),
            Rarity::Rare => write!(f, "Rare"),
            Rarity::Epic => write!(f, "Epic"),
            Rarity::Unique => write!(f, "Unique"),
        }
    }
}

#[derive(Event)]
struct DiePurchaseEvent(Die);

#[derive(Event)]
struct DieRolledEvent(Die);

#[derive(Event)]
struct DieRollResultEvent(Die, DieFace);

#[derive(Resource, Debug, Clone, Reflect)]
#[reflect(Resource)]
struct Die {
    // the faces of the die
    faces: Vec<DieFace>,
    // the current monetary value of the die
    value: usize,
    // result of the roll
    result: Option<DieFace>,
    // whether the die is currently being rolled
    rolling: bool,
}

impl PartialEq for Die {
    fn eq(&self, other: &Self) -> bool {
        self.faces == other.faces
    }
}

struct DieBuilder {
    base_face: DieFace,
    size: usize,
}

impl DieBuilder {
    pub fn from_d6_type(selected_type: BaseElementType) -> Self {
        DieBuilder {
            base_face: DieFace::new(selected_type, Rarity::Common),
            size: 6,
        }
    }

    fn build(self) -> Die {
        let mut faces = Vec::new();
        for _ in 0..self.size {
            faces.push(DieFace::generate(
                self.base_face.primary_type,
                self.base_face.rarity,
            ));
        }

        Die {
            faces,
            value: 20,
            result: None,
            rolling: false,
        }
    }
}

fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, gltfassets: Res<GltfAssets>) {
    commands.spawn((
        DirectionalLight {
            illuminance: light_consts::lux::OVERCAST_DAY,
            shadows_enabled: true,
            ..default()
        },
        Transform {
            translation: Vec3::new(0.0, 20.0, 0.0),
            rotation: Quat::from_rotation_x(-PI / 4.),
            ..default()
        },
        Name::new("Directional Light"),
    ));

    commands.spawn((
        SceneRoot(gltfassets.dungeon.clone()),
        RigidBody::Static,
        ColliderConstructorHierarchy::new(ColliderConstructor::TrimeshFromMesh),
    ));

    // spawn square placeholder for goal
    commands.spawn((
        Mesh3d(meshes.add(Rectangle::new(0.1, 1.0))),
        Transform::default()
            .with_translation(Vec3::new(-3.9, 0.0, -1.5))
            .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        Goal,
    ));

    commands.spawn((
        NavMeshSettings {
            // Define the outer borders of the navmesh.
            fixed: Triangulation::from_outer_edges(&[
                vec2(-20.0, -20.0),
                vec2(20.0, -20.0),
                vec2(20.0, 20.0),
                vec2(-20.0, 20.0),
            ]),
            ..default()
        },
        // Mark it for update as soon as obstacles are changed.
        // Other modes can be debounced or manually triggered.
        NavMeshUpdateMode::Debounced(0.2),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));
}

#[derive(Default, Component)]
struct Wave {
    timer: Timer,
}

fn die_purchased(
    mut die_pool: ResMut<GameResources>,
    mut ev_purchased: EventReader<DiePurchaseEvent>,
) {
    for ev in ev_purchased.read() {
        die_pool.dice.push(ev.0.clone());
    }
}
