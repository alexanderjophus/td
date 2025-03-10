use bevy::{gltf::GltfMesh, prelude::*};
use leafwing_input_manager::{prelude::*, Actionlike, InputControlKind};

use crate::{despawn_screen, GameState};

use super::{BaseElementType, GamePlayState, GameResources, TowerDetails, Wave, SNAP_OFFSET};

pub struct PlacementPlugin;

impl Plugin for PlacementPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(InputManagerPlugin::<PlacementAction>::default())
            .init_resource::<ActionState<PlacementAction>>()
            .insert_resource(PlacementAction::default_input_map())
            .add_systems(OnEnter(GamePlayState::Placement), setup)
            .add_systems(
                Update,
                (
                    control_cursor,
                    placeholder_snap_to_cursor,
                    display_placeholder,
                    toggle_placeholder_type,
                    // place_tower,
                    update_tower_selection,
                    start_wave,
                )
                    .run_if(in_state(GameState::Game).and(in_state(GamePlayState::Placement))),
            )
            .add_systems(
                OnExit(GamePlayState::Placement),
                despawn_screen::<PlacementOverlay>,
            );
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Hash, Debug, Reflect)]
enum PlacementAction {
    MoveCursorPlaceholder,
    ToggleTowerType,
    PlaceTower,
    EndPlacement,
}

impl Actionlike for PlacementAction {
    fn input_control_kind(&self) -> InputControlKind {
        match self {
            PlacementAction::MoveCursorPlaceholder => InputControlKind::DualAxis,
            PlacementAction::ToggleTowerType => InputControlKind::Button,
            PlacementAction::PlaceTower => InputControlKind::Button,
            PlacementAction::EndPlacement => InputControlKind::Button,
        }
    }
}

impl PlacementAction {
    /// Define the default bindings to the input
    fn default_input_map() -> InputMap<Self> {
        let mut input_map = InputMap::default();

        // Default gamepad input bindings
        input_map.insert_dual_axis(Self::MoveCursorPlaceholder, GamepadStick::RIGHT);
        input_map.insert(Self::ToggleTowerType, GamepadButton::East);
        input_map.insert(Self::PlaceTower, GamepadButton::South);
        input_map.insert(Self::EndPlacement, GamepadButton::West);

        // // Default kbm input bindings
        input_map.insert_dual_axis(Self::MoveCursorPlaceholder, VirtualDPad::arrow_keys());
        input_map.insert(Self::ToggleTowerType, KeyCode::KeyT);
        input_map.insert(Self::PlaceTower, KeyCode::Space);
        input_map.insert(Self::EndPlacement, KeyCode::Enter);

        input_map
    }
}

#[derive(Reflect, Component, PartialEq)]
#[reflect(Component)]
pub struct Tower {
    pub name: String,
    pub element_type: BaseElementType,
    pub attack_speed: Timer,
}

#[derive(Reflect, Component)]
#[reflect(Component)]
pub struct Projectile {
    pub speed: f32,
    pub damage: u32,
    pub target: Entity,
    pub lifetime: Timer,
}

#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct TowerPlaceholder;

#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct CursorPlaceholder;

#[derive(Reflect, Component)]
pub struct PlacementOverlay;

fn setup(
    mut commands: Commands,
    game_resources: ResMut<GameResources>,
    mut assets_mesh: ResMut<Assets<Mesh>>,
    assets_gltfmesh: Res<Assets<GltfMesh>>,
    assets_towers: Res<Assets<TowerDetails>>,
    res: Res<Assets<Gltf>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let tower = game_resources
        .towers
        .get(game_resources.highlighted_tower)
        .unwrap();
    let tower_details = assets_towers.get(*tower).unwrap();
    let gltf = res.get(&tower_details.model).unwrap();
    let mesh = assets_gltfmesh.get(&gltf.meshes[0]).unwrap();
    let mesh3d = mesh.primitives[0].mesh.clone();
    let mat = gltf.materials[0].clone();
    commands.spawn((
        Mesh3d(mesh3d),
        MeshMaterial3d(mat),
        Transform::default().with_translation(Vec3::new(SNAP_OFFSET, 0.0, SNAP_OFFSET)),
        TowerPlaceholder,
    ));

    let blue = materials.add(StandardMaterial {
        base_color: Color::srgb(0.0, 0.0, 1.0),
        ..Default::default()
    });

    commands.spawn((
        Mesh3d(assets_mesh.add(Cylinder::new(0.2, 0.05))),
        MeshMaterial3d(blue),
        Transform::default().with_translation(Vec3::new(SNAP_OFFSET, 0.0, SNAP_OFFSET)),
        CursorPlaceholder,
    ));

    let mut p = commands.spawn((
        Node {
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            top: Val::Percent(60.),
            ..default()
        },
        PlacementOverlay,
    ));
    p.with_child((
        Node {
            width: Val::Percent(40.),
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            ..default()
        },
        Text::new("Towers:"),
    ));

    for tower in game_resources.towers.iter() {
        let tower_details = assets_towers.get(*tower).unwrap();
        let idx = game_resources.highlighted_tower;
        p.with_child((
            Node {
                width: Val::Percent(20.),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(if tower == &game_resources.towers[idx] {
                Color::srgba(0., 0., 0., 0.5)
            } else {
                Color::srgba(0., 0., 0., 0.8)
            }),
            TowerDetails {
                name: tower_details.name.clone(),
                element_type: tower_details.element_type,
                model: tower_details.model.clone(),
            },
            Text::new(tower_details.name.clone()),
        ));
    }
}

fn control_cursor(
    time: Res<Time>,
    action_state: Res<ActionState<PlacementAction>>,
    mut query: Query<&mut Transform, With<CursorPlaceholder>>,
) {
    let mut player_transform = query.single_mut();
    let move_delta = time.delta_secs()
        * 2.0
        * action_state
            .clamped_axis_pair(&PlacementAction::MoveCursorPlaceholder)
            .xy();
    player_transform.translation += Vec3::new(move_delta.x, 0.0, -move_delta.y);
}

// snaps the tower placeholder to the nearest spot on the grid to the cursor
fn placeholder_snap_to_cursor(
    mut placeholder_query: Query<
        &mut Transform,
        (With<TowerPlaceholder>, Without<CursorPlaceholder>),
    >,
    cursor_query: Query<&mut Transform, (With<CursorPlaceholder>, Without<TowerPlaceholder>)>,
) {
    let cursor_transform = cursor_query.single();
    let cursor_position = cursor_transform.translation;

    placeholder_query
        .iter_mut()
        .for_each(|mut placeholder_transform| {
            let snap_distance = 1.0;
            let snap_x = (cursor_position.x - SNAP_OFFSET / snap_distance).round() + SNAP_OFFSET;
            let snap_z = (cursor_position.z - SNAP_OFFSET / snap_distance).round() + SNAP_OFFSET;

            placeholder_transform.translation.x = snap_x;
            placeholder_transform.translation.z = snap_z;
        });
}

fn toggle_placeholder_type(
    action_state: Res<ActionState<PlacementAction>>,
    mut game_resources: ResMut<GameResources>,
) {
    if action_state.just_pressed(&PlacementAction::ToggleTowerType) {
        game_resources.highlighted_tower =
            (game_resources.highlighted_tower + 1) % game_resources.towers.len();
    }
}

fn update_tower_selection(
    game_resources: Res<GameResources>,
    assets_towers: Res<Assets<TowerDetails>>,
    mut query: Query<(&mut BackgroundColor, &TowerDetails)>,
) {
    for (mut bg_color, item) in query.iter_mut() {
        let idx = game_resources.highlighted_tower;
        if let Some(tower) = game_resources.towers.get(idx) {
            let tower_details = assets_towers.get(*tower).unwrap();
            if item == tower_details {
                *bg_color = BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5));
            } else {
                *bg_color = BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8));
            }
        }
    }
}

fn display_placeholder(
    mut commands: Commands,
    game_resources: ResMut<GameResources>,
    assets_gltfmesh: Res<Assets<GltfMesh>>,
    assets_towers: Res<Assets<TowerDetails>>,
    res: Res<Assets<Gltf>>,
    mut query: Query<
        (&mut Mesh3d, &mut MeshMaterial3d<StandardMaterial>, Entity),
        With<TowerPlaceholder>,
    >,
) {
    if game_resources.towers.is_empty() {
        query.iter_mut().for_each(|(_, _, entity)| {
            commands.entity(entity).despawn_recursive();
        });
        return;
    }
    let idx = game_resources.highlighted_tower;
    if let Some(tower) = game_resources.towers.get(idx) {
        let tower_details = assets_towers.get(*tower).unwrap();
        let gltf = res.get(&tower_details.model).unwrap();
        let mesh = assets_gltfmesh.get(&gltf.meshes[0]).unwrap();
        query.iter_mut().for_each(|(mut mesh3d, mut mat, _)| {
            mesh3d.0 = mesh.primitives[0].mesh.clone();
            mat.0 = gltf.materials[0].clone();
        });
    }
}

// fn place_tower(
//     action_state: Res<ActionState<PlacementAction>>,
//     mut commands: Commands,
//     assets_towers: Res<Assets<TowerDetails>>,
//     res: Res<Assets<Gltf>>,
//     assets_gltfmesh: Res<Assets<GltfMesh>>,
//     mut tower_pool: ResMut<TowerPool>,
//     placeholder_query: Query<&Transform, With<TowerPlaceholder>>,
// ) {
//     if action_state.just_pressed(&PlacementAction::PlaceTower) {
//         let placeholder_transform = placeholder_query.single();
//         if let Some(tower) = tower_pool.get_highlighted() {
//             let tower_details = assets_towers.get(*tower).unwrap();
//             let gltf = res.get(&tower_details.model).unwrap();
//             let mesh = assets_gltfmesh.get(&gltf.meshes[0]).unwrap();
//             let mesh3d = mesh.primitives[0].mesh.clone();
//             let mat = gltf.materials[0].clone();
//             commands.spawn((
//                 Mesh3d(mesh3d),
//                 Transform::from_translation(placeholder_transform.translation),
//                 MeshMaterial3d(mat),
//                 Tower {
//                     name: tower_details.name.clone(),
//                     element_type: tower_details.element_type,
//                     attack_speed: Timer::from_seconds(1.0, TimerMode::Repeating),
//                 },
//                 Obstacle,
//             ));
//         }
//         tower_pool.remove_tower(tower_pool.get_highlighted().unwrap());
//     }
// }

fn start_wave(
    action_state: Res<ActionState<PlacementAction>>,
    mut next_state: ResMut<NextState<GamePlayState>>,
    mut commands: Commands,
) {
    if action_state.just_pressed(&PlacementAction::EndPlacement) {
        next_state.set(GamePlayState::Wave);
        commands.spawn(Wave {
            timer: Timer::from_seconds(20.0, TimerMode::Once),
        });
    }
}
