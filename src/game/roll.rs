use bevy::{
    color::palettes::css::{BLUE, GREEN, ORANGE, PURPLE, WHITE},
    prelude::*,
};
use leafwing_input_manager::{prelude::*, Actionlike, InputControlKind};

use crate::{despawn_screen, GameState};

use super::dice_physics::{DicePhysicsPlugin, ThrowPower};
use super::{Die, DieRollResultEvent, DieRolledEvent, GamePlayState, GameResources, Rarity};

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
            (
                handle_input,
                update_die_selection,
                update_die_result,
                save_die_result,
            )
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
    Throw,
    ThrowPower,
    Placement,
}

impl Actionlike for RollAction {
    fn input_control_kind(&self) -> InputControlKind {
        match self {
            RollAction::HighlightLeft => InputControlKind::Button,
            RollAction::HighlightRight => InputControlKind::Button,
            RollAction::Throw => InputControlKind::Button,
            RollAction::ThrowPower => InputControlKind::Axis,
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
        input_map.insert(Self::Throw, GamepadButton::East);
        input_map.insert_axis(Self::ThrowPower, GamepadAxis::LeftStickY);
        input_map.insert(Self::Placement, GamepadButton::South);

        // Default kbm input bindings
        input_map.insert(Self::HighlightLeft, KeyCode::ArrowLeft);
        input_map.insert(Self::HighlightRight, KeyCode::ArrowRight);
        input_map.insert(Self::Throw, KeyCode::Space);
        input_map.insert_axis(Self::ThrowPower, VirtualAxis::vertical_arrow_keys());
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

fn rolling_setup(mut commands: Commands, game_resources: Res<GameResources>) {
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
            for die in game_resources.dice.iter() {
                let mut n = parent.spawn((
                    Node {
                        width: Val::Percent(20.),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(
                        if die == &game_resources.dice[game_resources.highlighted_die] {
                            Color::srgba(0., 0., 0., 0.5)
                        } else {
                            Color::srgba(0., 0., 0., 0.8)
                        },
                    ),
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
    mut game_resources: ResMut<GameResources>,
    mut next_state: ResMut<NextState<GamePlayState>>,
    mut ev_rolled: EventWriter<DieRolledEvent>,
    mut throw_power: ResMut<ThrowPower>,
) {
    if action_state.just_pressed(&RollAction::HighlightLeft) {
        game_resources.highlighted_die =
            (game_resources.highlighted_die + game_resources.dice.len() - 1)
                % game_resources.dice.len();
    }

    if action_state.just_pressed(&RollAction::HighlightRight) {
        game_resources.highlighted_die =
            (game_resources.highlighted_die + 1) % game_resources.dice.len();
    }

    if action_state.just_pressed(&RollAction::Throw) {
        let idx = game_resources.highlighted_die;
        // Only trigger the roll if there's no result yet
        if game_resources.dice[idx].result.is_none() && !game_resources.dice[idx].rolling {
            game_resources.dice[idx].rolling = true;
            // The physics system will handle the actual rolling
            ev_rolled.send(DieRolledEvent(game_resources.dice[idx].clone()));
        }
    }

    throw_power.0 = action_state.clamped_value(&RollAction::ThrowPower);

    if action_state.just_pressed(&RollAction::Placement) {
        if game_resources.towers.is_empty() {
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
            if item.die == ev.0 {
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
    game_resources: Res<GameResources>,
    mut query: Query<(&mut BackgroundColor, &DieItem)>,
) {
    for (mut bg, item) in query.iter_mut() {
        *bg = BackgroundColor(
            if item.die == game_resources.dice[game_resources.highlighted_die] {
                Color::srgba(0., 0., 0., 0.5)
            } else {
                Color::srgba(0., 0., 0., 0.8)
            },
        );
    }
}

fn save_die_result(
    mut game_resources: ResMut<GameResources>,
    mut ev_result: EventReader<DieRollResultEvent>,
) {
    for ev in ev_result.read() {
        // Find the die with matching ID and update its result
        for die in game_resources.dice.iter_mut() {
            if *die == ev.0 {
                die.result = Some(ev.1);
                break;
            }
        }
    }
}
