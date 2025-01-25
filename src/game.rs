mod camera;
mod economy;
mod placement;
mod roll;
mod wave;

use super::GameState;
use bevy::gltf::GltfMesh;
use bevy::math::vec2;
use bevy::prelude::*;
use bevy::utils::HashMap;
use bevy::{ecs::system::SystemState, gltf::Gltf, render::primitives::Aabb};
use bevy_asset_loader::prelude::*;
use bevy_common_assets::ron::RonAssetPlugin;
use camera::CameraPlugin;
use economy::EconomyPlugin;
use placement::PlacementPlugin;
use rand::seq::{IteratorRandom, SliceRandom};
use rand::{thread_rng, Rng};
use roll::RollPlugin;
use std::f32::consts::PI;
use std::time::Duration;
use vleue_navigator::prelude::*;
use wave::{EnemySpawner, WavePlugin};

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
                RonAssetPlugin::<AssetCollections>::new(&["game.ron"]),
                VleueNavigatorPlugin,
                NavmeshUpdaterPlugin::<Aabb, Obstacle>::default(),
            ))
            .init_resource::<Assets<TowerDetails>>()
            .init_resource::<Assets<EnemyDetails>>()
            .init_resource::<DiePool>()
            .init_resource::<TowerPool>()
            .insert_resource(DiePool {
                dice: Vec::new(),
                highlighted: 0,
            })
            .insert_resource(TowerPool {
                towers: Vec::new(),
                highlighted: 0,
            })
            .register_type::<DiePool>()
            .add_event::<DiePurchaseEvent>()
            .add_event::<DieRolledEvent>()
            .add_event::<DieRollResultEvent>()
            .add_systems(OnEnter(GameState::Game), setup)
            .add_systems(
                Update,
                (die_purchased, die_rolled).run_if(in_state(GameState::Game)),
            );
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
pub struct AllAssets {
    #[asset(key = "towers", collection(typed))]
    pub towers: Vec<Handle<TowerDetails>>,
    #[asset(key = "enemies", collection(typed))]
    pub enemies: Vec<Handle<EnemyDetails>>,
}

/// Representation of a loaded tower file.
#[derive(Asset, Resource, Debug, PartialEq, Clone, TypePath)]
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
                        element_type: tower.element_type.clone(),
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
    #[asset(path = "models/house.glb")]
    pub house: Handle<Gltf>,
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

#[derive(Event)]
struct DiePurchaseEvent(Die);

#[derive(Event)]
struct DieRolledEvent(Die);

#[derive(Event)]
struct DieRollResultEvent(Die, DieFace);

#[derive(Resource, Debug, Clone, PartialEq, Reflect)]
#[reflect(Resource)]
struct Die {
    // the faces of the die
    faces: Vec<DieFace>,
    // the current monetary value of the die
    value: usize,
    // result of the roll
    result: Option<DieFace>,
}

impl Die {
    fn roll(&mut self) -> DieFace {
        let mut rng = rand::thread_rng();
        let face = rng.gen_range(0..self.faces.len());
        let res = self.faces[face];
        self.result = Some(res);
        self.faces[face].clone()
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

        // the value should;
        // have a base value depending on number of faces
        // grow with the rarity on each face

        Die {
            faces,
            value: 20,
            result: None,
        }
    }
}

#[derive(Resource, Default, Debug, Clone, PartialEq, Reflect)]
#[reflect(Resource)]
struct DiePool {
    dice: Vec<Die>,
    highlighted: usize,
}

#[derive(Resource, Default, Debug, PartialEq)]
struct TowerPool {
    towers: Vec<AssetId<TowerDetails>>,
    highlighted: usize,
}

impl TowerPool {
    fn toggle_highlighted(&mut self) {
        if self.towers.is_empty() {
            return;
        }
        self.highlighted = (self.highlighted + 1) % self.towers.len();
    }
}

fn setup(
    mut commands: Commands,
    assets_gltfmesh: Res<Assets<GltfMesh>>,
    mut assets_mesh: ResMut<Assets<Mesh>>,
    assets_enemydetails: Res<Assets<EnemyDetails>>,
    gltfassets: Res<GltfAssets>,
    res: Res<Assets<Gltf>>,
) {
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

    // get first enemy from assets
    let enemy = assets_enemydetails.iter().next().unwrap();
    let enemy_mesh = res.get(&enemy.1.model).unwrap();
    let enemy_mesh_mesh = assets_gltfmesh.get(&enemy_mesh.meshes[0]).unwrap();

    commands.spawn((
        Mesh3d(enemy_mesh_mesh.primitives[0].mesh.clone()),
        MeshMaterial3d(enemy_mesh.materials[0].clone()),
        Transform::from_translation(Vec3::new(0.5, 0.0, -10.0)),
        EnemySpawner {
            timer: Timer::new(Duration::from_secs(1), TimerMode::Repeating),
        },
    ));

    let house_mesh = res.get(&gltfassets.house).unwrap();
    let house_mesh_mats = assets_gltfmesh.get(&house_mesh.meshes[0]).unwrap();

    commands.spawn((
        Mesh3d(house_mesh_mats.primitives[0].mesh.clone()),
        MeshMaterial3d(house_mesh.materials[0].clone()),
        Transform::from_translation(Vec3::new(-5.8, 0.0, -4.0))
            .with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2))
            .with_scale(Vec3::splat(0.25)),
        Obstacle,
        Name::new("House"),
    ));

    // spawn square placeholder for goal
    commands.spawn((
        Mesh3d(assets_mesh.add(Rectangle::new(0.1, 1.0))),
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
        NavMeshUpdateMode::Direct,
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));
}

#[derive(Default, Component)]
struct Wave {
    timer: Timer,
}

fn die_purchased(mut die_pool: ResMut<DiePool>, mut ev_purchased: EventReader<DiePurchaseEvent>) {
    for ev in ev_purchased.read() {
        die_pool.dice.push(ev.0.clone());
    }
}

fn die_rolled(
    tower_assets: Res<Assets<TowerDetails>>,
    mut die_pool: ResMut<DiePool>,
    mut tower_pool: ResMut<TowerPool>,
    mut ev_rolled: EventReader<DieRolledEvent>,
    mut ev_result: EventWriter<DieRollResultEvent>,
) {
    for ev in ev_rolled.read() {
        let mut die = ev.0.clone();
        let face = die.roll();
        let idx = die_pool.highlighted;
        die_pool.dice[idx] = die.clone();
        let selected_type = face.primary_type.clone();
        let (id, _) = tower_assets
            .iter()
            .filter(|(_, tower)| tower.element_type == selected_type)
            .choose(&mut rand::thread_rng())
            .unwrap();
        tower_pool.towers.push(id);
        ev_result.send(DieRollResultEvent(die, face));
    }
}
