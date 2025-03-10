use bevy::{
    color::palettes::css::{BLUE, GREEN, ORANGE, PURPLE, WHITE},
    prelude::*,
};
use leafwing_input_manager::{prelude::*, Actionlike, InputControlKind};

use crate::{despawn_screen, GameState};

use super::{
    BaseElementType, Die, DieBuilder, DiePurchaseEvent, GamePlayState, GameResources, Rarity,
};

pub struct EconomyPlugin;

impl Plugin for EconomyPlugin {
    fn build(&self, app: &mut App) {
        let shop_items = vec![
            DieBuilder::from_d6_type(BaseElementType::Fire).build(),
            DieBuilder::from_d6_type(BaseElementType::Water).build(),
            DieBuilder::from_d6_type(BaseElementType::Earth).build(),
            DieBuilder::from_d6_type(BaseElementType::Wind).build(),
        ];
        app.add_plugins(InputManagerPlugin::<EconomyAction>::default())
            .init_resource::<ActionState<EconomyAction>>()
            .insert_resource(EconomyAction::default_input_map())
            .insert_resource(DieShop {
                highlighted: 0,
                items: shop_items,
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

fn economy_setup(mut commands: Commands, shop: Res<DieShop>, economy: ResMut<GameResources>) {
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
    mut game_resources: ResMut<GameResources>,
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
        if game_resources.money < cost
            || game_resources.dice.contains(&shop.items[shop.highlighted])
        {
            return;
        }
        game_resources.money -= cost;
        let idx = shop.highlighted;
        game_resources.dice.push(shop.items[idx].clone());
        ev_die_purchase.send(DiePurchaseEvent(shop.items[idx].clone()));
    }
}

fn update_shop_ui(
    shop: ResMut<DieShop>,
    game_resources: Res<GameResources>,
    mut query: Query<(&mut BackgroundColor, &DieShopItem, Entity), Without<Text>>,
    text_query: Query<&mut Text>,
    mut commands: Commands,
) {
    for (mut bg_color, item, entity) in query.iter_mut() {
        // First update background colors for selection highlighting
        if item.item == shop.items[shop.highlighted] {
            *bg_color = BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5));
        } else {
            *bg_color = BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8));
        }

        // Check if this item is purchased and needs the "Purchased" label
        let is_purchased = game_resources.dice.contains(&item.item);

        // Check if this item already has the "Purchased" label
        let has_purchased_label = text_query
            .get(entity)
            .map(|text| text.0 == "Purchased")
            .unwrap_or(false);

        // Add "Purchased" label if needed and not already present
        if is_purchased && !has_purchased_label {
            commands.entity(entity).with_children(|parent| {
                parent.spawn((
                    Node {
                        width: Val::Percent(80.),
                        height: Val::Px(30.),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.5, 0.0, 0.0, 0.7)),
                    Text::new("Purchased"),
                    TextColor(WHITE.into()),
                ));
            });
        }
    }
}

fn update_economy_ui(
    game_resources: Res<GameResources>,
    mut query: Query<&mut Text, With<MoneyText>>,
) {
    for mut text in query.iter_mut() {
        text.0 = game_resources.money.to_string();
    }
}

fn start_rolling(
    action_state: Res<ActionState<EconomyAction>>,
    mut next_state: ResMut<NextState<GamePlayState>>,
    game_resources: Res<GameResources>,
) {
    if action_state.just_pressed(&EconomyAction::PlacementPhase) {
        if game_resources.dice.is_empty() {
            return;
        }
        next_state.set(GamePlayState::Rolling);
    }
}
