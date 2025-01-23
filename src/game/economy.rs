use bevy::{
    color::palettes::css::{BLUE, GREEN, ORANGE, PURPLE, WHITE},
    prelude::*,
};
use leafwing_input_manager::{prelude::*, Actionlike, InputControlKind};

use crate::{despawn_screen, GameState};

use super::{BaseElementType, Die, DieBuilder, DiePurchaseEvent, GamePlayState, Rarity};

pub struct EconomyPlugin;

impl Plugin for EconomyPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(InputManagerPlugin::<EconomyAction>::default())
            .init_resource::<ActionState<EconomyAction>>()
            .insert_resource(EconomyAction::default_input_map())
            .insert_resource(Economy { money: 50 })
            .insert_resource(DieShop {
                highlighted: 0,
                items: vec![
                    DieBuilder::from_d6_type(BaseElementType::Fire).build(),
                    DieBuilder::from_d6_type(BaseElementType::Water).build(),
                    DieBuilder::from_d6_type(BaseElementType::Earth).build(),
                    DieBuilder::from_d6_type(BaseElementType::Wind).build(),
                ],
            })
            .add_systems(OnEnter(GameState::Game), economy_setup)
            .add_systems(
                Update,
                (choose_die, update_shop_ui, start_rolling, update_economy_ui)
                    .run_if(in_state(GamePlayState::Economy).and(in_state(GameState::Game))),
            )
            .add_systems(
                OnExit(GamePlayState::Economy),
                despawn_screen::<DieShopOverlay>,
            );
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Hash, Debug, Reflect, Resource)]
#[reflect(Resource)]
enum EconomyAction {
    ToggleDieLeft,
    ToggleDieRight,
    BuyDie,
    PlacementPhase,
}

impl Actionlike for EconomyAction {
    fn input_control_kind(&self) -> InputControlKind {
        match self {
            EconomyAction::ToggleDieLeft => InputControlKind::Button,
            EconomyAction::ToggleDieRight => InputControlKind::Button,
            EconomyAction::BuyDie => InputControlKind::Button,
            EconomyAction::PlacementPhase => InputControlKind::Button,
        }
    }
}

impl EconomyAction {
    /// Define the default bindings to the input
    fn default_input_map() -> InputMap<Self> {
        let mut input_map = InputMap::default();

        // Default gamepad input bindings
        input_map.insert(Self::ToggleDieLeft, GamepadButton::DPadLeft);
        input_map.insert(Self::ToggleDieRight, GamepadButton::DPadRight);
        input_map.insert(Self::BuyDie, GamepadButton::East);
        input_map.insert(Self::PlacementPhase, GamepadButton::South);

        // Default kbm input bindings
        input_map.insert(Self::ToggleDieLeft, KeyCode::ArrowLeft);
        input_map.insert(Self::ToggleDieRight, KeyCode::ArrowRight);
        input_map.insert(Self::BuyDie, KeyCode::Space);
        input_map.insert(Self::PlacementPhase, KeyCode::Enter);

        input_map
    }
}

#[derive(Resource)]
pub struct Economy {
    pub money: usize,
}

#[derive(Resource, Debug, Clone, PartialEq)]
struct DieShop {
    items: Vec<Die>,
    highlighted: usize,
}

#[derive(Component)]
struct DieShopOverlay;

#[derive(Component)]
struct DieShopItem {
    item: Die,
}

#[derive(Component)]
struct MoneyText;

fn economy_setup(mut commands: Commands, shop: Res<DieShop>, economy: ResMut<Economy>) {
    let mut p = commands.spawn((
        Node {
            width: Val::Percent(100.),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            top: Val::Percent(60.),
            ..default()
        },
        DieShopOverlay,
    ));
    p.with_child((
        Node {
            width: Val::Percent(10.),
            height: Val::Percent(10.),
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            ..default()
        },
        Text::new("Money:"),
    ));
    p.with_child((
        Node {
            width: Val::Percent(10.),
            height: Val::Percent(10.),
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            ..default()
        },
        Text::new(economy.money.to_string()),
        MoneyText,
    ));
    for item in shop.items.iter() {
        p.with_children(|p| {
            let mut die = p.spawn((
                Node {
                    width: Val::Percent(20.),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BackgroundColor(if item == &shop.items[shop.highlighted] {
                    Color::srgba(0., 0., 0., 0.5)
                } else {
                    Color::srgba(0., 0., 0., 0.8)
                }),
                DieShopItem { item: item.clone() },
            ));
            for face in item.faces.iter() {
                let color = match face.rarity {
                    Rarity::Common => WHITE,
                    Rarity::Uncommon => GREEN,
                    Rarity::Rare => BLUE,
                    Rarity::Epic => PURPLE,
                    Rarity::Unique => ORANGE,
                };
                die.with_child((
                    Node::default(),
                    Text::new(face.primary_type.to_string()),
                    TextColor(color.into()),
                ));
            }
        });
    }
}

fn choose_die(
    action_state: Res<ActionState<EconomyAction>>,
    mut economy: ResMut<Economy>,
    mut shop: ResMut<DieShop>,
    mut ev_die_purchase: EventWriter<DiePurchaseEvent>,
) {
    if action_state.just_pressed(&EconomyAction::ToggleDieLeft) {
        shop.highlighted = (shop.highlighted + shop.items.len() - 1) % shop.items.len();
    }
    if action_state.just_pressed(&EconomyAction::ToggleDieRight) {
        shop.highlighted = (shop.highlighted + 1) % shop.items.len();
    }
    // Buy the die, remove costs, add to diepool resource
    if action_state.just_pressed(&EconomyAction::BuyDie) {
        let cost = shop.items[shop.highlighted].value;
        if economy.money < cost {
            return;
        }
        economy.money -= cost;
        ev_die_purchase.send(DiePurchaseEvent(shop.items[shop.highlighted].clone()));
    }
}

fn update_shop_ui(shop: ResMut<DieShop>, mut query: Query<(&mut BackgroundColor, &DieShopItem)>) {
    for (mut bg_color, item) in query.iter_mut() {
        if item.item == shop.items[shop.highlighted] {
            *bg_color = BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5));
        } else {
            *bg_color = BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8));
        }
    }
}

fn update_economy_ui(economy: Res<Economy>, mut query: Query<&mut Text, With<MoneyText>>) {
    for mut text in query.iter_mut() {
        text.0 = economy.money.to_string();
    }
}

fn start_rolling(
    action_state: Res<ActionState<EconomyAction>>,
    mut next_state: ResMut<NextState<GamePlayState>>,
) {
    if action_state.just_pressed(&EconomyAction::PlacementPhase) {
        next_state.set(GamePlayState::Rolling);
    }
}
