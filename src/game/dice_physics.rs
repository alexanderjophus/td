use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use std::f32::consts::PI;
use std::time::Duration;

use crate::despawn_screen;

use super::{camera::FollowCam, Die, DieRollResultEvent, DieRolledEvent, GamePlayState};

pub struct DicePhysicsPlugin;

impl Plugin for DicePhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
            .add_plugins(RapierDebugRenderPlugin::default().disabled())
            .insert_resource(ThrowPower(0.2))
            .add_systems(OnEnter(GamePlayState::Rolling), setup_throwing_area)
            .add_systems(
                Update,
                (
                    handle_dice_roll,
                    check_dice_result,
                    cleanup_dice,
                    update_throw_indicator,
                )
                    .run_if(in_state(GamePlayState::Rolling)),
            )
            .add_systems(OnExit(GamePlayState::Rolling), despawn_screen::<OnDieRoll>);
    }
}

// Component to mark the physical die entity
#[derive(Component)]
struct PhysicalDie {
    die_data: Die,
    is_rolling: bool,
    face_up: Option<usize>,
    roll_timeout_timer: Timer,
    roll_display_timer: Timer,
}

// Component for the rolling platform
#[derive(Component)]
struct OnDieRoll;

// Component for the throw indicator
#[derive(Component)]
struct ThrowIndicator;

// Resource for tracking throw parameters
#[derive(Resource)]
pub struct ThrowPower(pub f32);

// Setup the throwing area
fn setup_throwing_area(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // // Initialize throw parameters with default value
    // commands.insert_resource(ThrowPower(0.2));

    // Create a visual indicator for throw direction and power
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.05, 2.0))), // Thin arrow-like cylinder
        MeshMaterial3d(materials.add(Color::srgba(1.0, 0.5, 0.0, 0.5))), // Semi-transparent orange
        Transform::from_xyz(0.0, 0.1, 0.0) // Just above the ground
            .with_rotation(Quat::from_rotation_x(PI / 2.0)), // Point forward
        ThrowIndicator,
        OnDieRoll,
    ));
}

// Update the throw indicator based on current throw parameters
fn update_throw_indicator(
    throw_power: Res<ThrowPower>,
    mut indicator_query: Query<&mut Transform, With<ThrowIndicator>>,
) {
    if let Ok(mut transform) = indicator_query.get_single_mut() {
        // Update length based on power
        let scale = Vec3::new(1.0, throw_power.0 * 3.0, 1.0); // Scale the y-axis of the cylinder
        transform.scale = scale;
    }
}

// Modified handle_dice_roll to use throw parameters
fn handle_dice_roll(
    mut commands: Commands,
    mut ev_rolled: EventReader<DieRolledEvent>,
    camera_query: Query<&Transform, With<FollowCam>>,
    throw_power: Res<ThrowPower>,
    asset_server: Res<AssetServer>,
) {
    let transform = camera_query.single();
    for ev in ev_rolled.read() {
        let die_data = ev.0.clone();

        // Calculate 3D throw direction from 2D input
        let direction = Vec3::new(0.0, 0.5, -1.0).normalize();

        // Scale by power
        let throw_force = direction * (throw_power.0 * 10.0 + 5.0); // Base power + scaling

        // Calculate a reasonable angular velocity
        // should be spinning on a random axis
        let spin_axis = Vec3::new(
            rand::random::<f32>() - 0.5,
            rand::random::<f32>() - 0.5,
            rand::random::<f32>() - 0.5,
        );
        let angular_velocity = spin_axis * (throw_power.0 * PI * 4.0 + PI); // Base spin + scaling

        // Create the die with physics
        commands.spawn((
            SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/die.glb"))),
            Transform {
                translation: transform.translation + transform.forward() * 2.0,
                rotation: transform.rotation,
                scale: Vec3::splat(0.5),
            },
            RigidBody::Dynamic,
            Collider::cuboid(0.5, 0.5, 0.5),
            Restitution::coefficient(0.7), // Bounciness
            Friction::coefficient(0.5),
            // Apply initial velocities
            Velocity {
                linvel: throw_force,
                angvel: angular_velocity,
            },
            PhysicalDie {
                die_data,
                is_rolling: true,
                face_up: None,
                roll_timeout_timer: Timer::new(Duration::from_secs(5), TimerMode::Once),
                roll_display_timer: Timer::new(Duration::from_secs(2), TimerMode::Once),
            },
            OnDieRoll,
        ));
    }
}

// System to check when the die has stopped rolling and determine the result
fn check_dice_result(
    mut dice_query: Query<(&mut PhysicalDie, &Transform, &Velocity)>,
    mut ev_result: EventWriter<DieRollResultEvent>,
    time: Res<Time>,
) {
    for (mut physical_die, transform, velocity) in dice_query.iter_mut() {
        // Always tick the timer
        physical_die.roll_timeout_timer.tick(time.delta());

        if !physical_die.is_rolling {
            continue;
        }

        // Check if the die has stopped moving (almost) or timer expired
        let linear_threshold = 0.1;
        let angular_threshold = 0.1;

        if (velocity.linvel.length() < linear_threshold
            && velocity.angvel.length() < angular_threshold)
            || physical_die.roll_timeout_timer.finished()
        {
            // Die has stopped, determine which face is up
            // Get the up vector in local dice space
            let up = Vec3::Y;

            // Determine which face is most aligned with up
            let face_index = determine_face_up(transform, up);

            physical_die.is_rolling = false;
            physical_die.face_up = Some(face_index);

            // Send the result event
            let face = physical_die.die_data.faces[face_index];
            ev_result.send(DieRollResultEvent(physical_die.die_data.clone(), face));

            physical_die.roll_display_timer.reset();
        }
    }
}

// Helper function to determine which face is up based on the die's orientation
fn determine_face_up(transform: &Transform, up: Vec3) -> usize {
    // Get die local axes in world space
    let local_x = transform.rotation * Vec3::X;
    let local_y = transform.rotation * Vec3::Y;
    let local_z = transform.rotation * Vec3::Z;

    // Calculate dot products with world up vector
    let dot_x = local_x.dot(up).abs();
    let dot_y = local_y.dot(up).abs();
    let dot_z = local_z.dot(up).abs();

    // Find which axis is most aligned with up
    if dot_x > dot_y && dot_x > dot_z {
        // X axis is most aligned with up
        if local_x.dot(up) > 0.0 {
            0
        } else {
            1
        }
    } else if dot_y > dot_x && dot_y > dot_z {
        // Y axis is most aligned with up
        if local_y.dot(up) > 0.0 {
            2
        } else {
            3
        }
    } else {
        // Z axis is most aligned with up
        if local_z.dot(up) > 0.0 {
            4
        } else {
            5
        }
    }
}

// Cleanup system for dice after roll is complete
fn cleanup_dice(
    mut commands: Commands,
    mut dice_query: Query<(Entity, &mut PhysicalDie)>,
    time: Res<Time>,
) {
    for (entity, mut die) in dice_query.iter_mut() {
        die.roll_display_timer.tick(time.delta());
        if !die.is_rolling && die.roll_display_timer.just_finished() {
            commands.entity(entity).despawn_recursive();
        }
    }
}
