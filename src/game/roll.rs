use bevy::{
    color::palettes::css::{BLUE, GREEN, ORANGE, PURPLE, WHITE},
    prelude::*,
};
use leafwing_input_manager::{prelude::*, Actionlike, InputControlKind};

use crate::{despawn_screen, GameState};

use super::dice_physics::DicePhysicsPlugin;
use super::{Die, DiePool, DieRollResultEvent, DieRolledEvent, GamePlayState, Rarity, TowerPool};

pub struct RollPlugin;

impl Plugin for RollPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            InputManagerPlugin::<RollAction>::default(),
            DicePhysicsPlugin,
        ))
        .init_resource::<ActionState<RollAction>>()
        .insert_resource(RollAction::default_input_map())
        .add_systems(OnEnter(GamePlayState::Rolling), rolling_setup)
        .add_systems(
            Update,
            (handle_input, update_die_selection, update_die_result)
                .run_if(in_state(GameState::Game).and(in_state(GamePlayState::Rolling))),
        )
        .add_systems(
            OnExit(GamePlayState::Rolling),
            despawn_screen::<DieRollingOverlay>,
        );
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Hash, Debug, Reflect, Resource)]
#[reflect(Resource)]
enum RollAction {
    HighlightLeft,
    HighlightRight,
    Roll,
    Placement,
}

impl Actionlike for RollAction {
    fn input_control_kind(&self) -> InputControlKind {
        match self {
            RollAction::HighlightLeft => InputControlKind::Button,
            RollAction::HighlightRight => InputControlKind::Button,
            RollAction::Roll => InputControlKind::Button,
            RollAction::Placement => InputControlKind::Button,
        }
    }
}

impl RollAction {
    /// Define the default bindings to the input
    fn default_input_map() -> InputMap<Self> {
        let mut input_map = InputMap::default();

        // Default gamepad input bindings
        input_map.insert(Self::HighlightLeft, GamepadButton::DPadLeft);
        input_map.insert(Self::HighlightRight, GamepadButton::DPadRight);
        input_map.insert(Self::Roll, GamepadButton::East);
        input_map.insert(Self::Placement, GamepadButton::South);

        // Default kbm input bindings
        input_map.insert(Self::HighlightLeft, KeyCode::ArrowLeft);
        input_map.insert(Self::HighlightRight, KeyCode::ArrowRight);
        input_map.insert(Self::Roll, KeyCode::Space);
        input_map.insert(Self::Placement, KeyCode::Enter);

        input_map
    }
}

#[derive(Component)]
struct DieRollingOverlay;

#[derive(Component)]
struct DieItem {
    die: Die,
}

fn rolling_setup(mut commands: Commands, die_pool: Res<DiePool>) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                top: Val::Percent(60.),
                ..default()
            },
            DieRollingOverlay,
        ))
        .with_children(|parent| {
            parent.spawn((
                Node {
                    width: Val::Percent(40.),
                    ..default()
                },
                Text::new("Rolling: Choose a die"),
            ));
            for die in die_pool.dice.iter() {
                let mut n = parent.spawn((
                    Node {
                        width: Val::Percent(20.),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(if die == &die_pool.dice[die_pool.highlighted] {
                        Color::srgba(0., 0., 0., 0.5)
                    } else {
                        Color::srgba(0., 0., 0., 0.8)
                    }),
                    DieItem { die: die.clone() },
                ));
                for face in die.faces.iter() {
                    n.with_child((
                        Node::default(),
                        Text::new(face.primary_type.to_string()),
                        TextColor(match face.rarity {
                            Rarity::Common => WHITE.into(),
                            Rarity::Uncommon => GREEN.into(),
                            Rarity::Rare => BLUE.into(),
                            Rarity::Epic => PURPLE.into(),
                            Rarity::Unique => ORANGE.into(),
                        }),
                    ));
                }
            }
        });
}

fn handle_input(
    action_state: Res<ActionState<RollAction>>,
    mut die_pool: ResMut<DiePool>,
    mut next_state: ResMut<NextState<GamePlayState>>,
    mut ev_rolled: EventWriter<DieRolledEvent>,
    tower_pool: Res<TowerPool>,
) {
    if action_state.just_pressed(&RollAction::HighlightLeft) {
        die_pool.highlighted =
            (die_pool.highlighted + die_pool.dice.len() - 1) % die_pool.dice.len();
    }

    if action_state.just_pressed(&RollAction::HighlightRight) {
        die_pool.highlighted = (die_pool.highlighted + 1) % die_pool.dice.len();
    }

    if action_state.just_pressed(&RollAction::Roll) {
        let idx = die_pool.highlighted;
        // Only trigger the roll if there's no result yet
        if die_pool.dice[idx].result.is_none() {
            // The physics system will handle the actual rolling
            ev_rolled.send(DieRolledEvent(die_pool.dice[idx].clone()));
        }
    }

    if action_state.just_pressed(&RollAction::Placement) {
        if tower_pool.towers.is_empty() {
            return;
        }
        next_state.set(GamePlayState::Placement);
    }
}

fn update_die_result(
    mut commands: Commands,
    mut query: Query<(Entity, &DieItem)>,
    mut ev_rolled: EventReader<DieRollResultEvent>,
) {
    for ev in ev_rolled.read() {
        for (entity, item) in query.iter_mut() {
            if item.die.faces == ev.0.faces {
                commands.entity(entity).despawn_descendants();
                commands.entity(entity).with_child((
                    Node::default(),
                    Text::new(ev.1.primary_type.to_string()),
                    TextColor(match ev.1.rarity {
                        Rarity::Common => WHITE.into(),
                        Rarity::Uncommon => GREEN.into(),
                        Rarity::Rare => BLUE.into(),
                        Rarity::Epic => PURPLE.into(),
                        Rarity::Unique => ORANGE.into(),
                    }),
                ));
            }
        }
    }
}

fn update_die_selection(
    die_pool: Res<DiePool>,
    mut query: Query<(&mut BackgroundColor, &DieItem)>,
) {
    for (mut bg, item) in query.iter_mut() {
        *bg = BackgroundColor(if item.die == die_pool.dice[die_pool.highlighted] {
            Color::srgba(0., 0., 0., 0.5)
        } else {
            Color::srgba(0., 0., 0., 0.8)
        });
    }
}
