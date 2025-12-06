use std::time::Duration;

use bevy_app::{Plugin, PreStartup, Update};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    children,
    component::Component,
    entity::Entity,
    event::{EntityEvent, Event},
    lifecycle::HookContext,
    observer::On,
    query::{AnyOf, Has, QueryData},
    resource::Resource,
    system::{Commands, Query, Res},
    world::World,
};

use bevy_ecs::spawn::SpawnRelated;

use bevy_log::error;
use bevy_math::{Rect, Vec2};
use bevy_picking::{
    Pickable,
    events::{Move, Out, Pointer, Press},
    pointer::PointerButton,
};
use bevy_platform::collections::HashMap;
use bevy_text::TextSpan;
use bevy_time::{Time, Timer, TimerMode};
use bevy_ui::{
    Display, GlobalZIndex, GridAutoFlow, Node, PositionType, RelativeCursorPosition, UiRect, Val,
    widget::Text,
};
use bevy_window::Window;
use tiny_bail::prelude::*;

use crate::{
    events::TooltipLocked,
    highlight::{HighlightPlugin, TooltipHighlightLink},
    layout::{TooltipStringText, TooltipTextNode, TooltipTitleNode, TooltipTitleText},
    text_observer::{
        TextHoveredOut, TextHoveredOver, TextMiddlePress, TextObservePlugin, WasHoveringText,
    },
};

pub mod events;
pub mod highlight;
pub mod layout;
pub mod text_observer;

pub struct NestedTooltipPlugin;

impl Plugin for NestedTooltipPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_plugins(TextObservePlugin)
            .add_plugins(HighlightPlugin)
            .init_resource::<TooltipConfiguration>()
            .init_resource::<TooltipReference>()
            .add_systems(PreStartup, setup_component_hooks)
            .add_systems(Update, tick_timers)
            .add_observer(spawn_time_done);
    }
}

/// Resource that configures the behaviour of tooltips
#[derive(Resource, Debug)]
pub struct TooltipConfiguration {
    /// See the `ActivationMethod` variants
    pub activation_method: ActivationMethod,

    /// Maximum amount of time the `ToolTip` will remain around without user interaction
    pub interaction_wait_for_time: Duration,

    /// The starting z_index this will be incremented for each recursive tooltip
    /// increase this if tooltips are not on top and you want to fix that
    pub starting_z_index: i32,
}

impl Default for TooltipConfiguration {
    fn default() -> Self {
        Self {
            activation_method: Default::default(),
            interaction_wait_for_time: Duration::from_secs_f64(0.5),
            starting_z_index: 3,
        }
    }
}

/// How a tooltip is triggered by default this is done via hovering
#[derive(Debug, Clone)]
pub enum ActivationMethod {
    /// Middle mouse button is pressed
    MiddleMouse,
    /// Mouse is over the `Tooltip` for a duration
    Hover { time: Duration },
}

impl Default for ActivationMethod {
    fn default() -> Self {
        ActivationMethod::Hover {
            time: Duration::from_secs_f64(0.9),
        }
    }
}

/// Default node for the `Tooltip` node use this to layout your tooltips without
/// accidentally moving it's position
#[derive(Resource, Debug)]
pub struct TooltipReference {
    /// Top level Node this will be copied to the `Tooltip` positions will be overwritten
    tooltip_node: Node,
}

impl TooltipReference {
    pub fn new(tooltip_node: Node) -> Self {
        Self { tooltip_node }
    }
}

impl Default for TooltipReference {
    fn default() -> Self {
        Self {
            tooltip_node: Node {
                position_type: PositionType::Absolute,
                display: Display::Grid,
                grid_auto_flow: GridAutoFlow::Row,
                max_width: Val::Vw(35.),
                min_height: Val::Vh(5.),
                max_height: Val::Vh(20.),
                border: UiRect::all(Val::Px(1.)),
                ..Default::default()
            },
        }
    }
}

/// Indicates this entity is a tooltip and stores what spawned it
/// The entity that spawned it is blocked from spawning another tooltip
/// until this one is finished to prevent tooltip jumping around
#[derive(Debug, Component)]
#[require(RelativeCursorPosition)]
pub struct Tooltip {
    entity: Entity,
}

impl Tooltip {
    /// The entity that spawned this tooltip
    pub fn entity(&self) -> Entity {
        self.entity
    }
}

/// When the cursor has gotten sufficently inside the tooltip
/// leaving will now despawn this tooltip
#[derive(Debug, Component)]
struct ToolTipDebounced;

#[derive(Debug, EntityEvent)]
pub struct TooltipSpawned {
    pub entity: Entity,
}

/// If the user hasn't hovered on the tooltip in the specified time despawn it
/// time is configured in `TooltipConfiguration`
#[derive(Debug, Component)]
pub struct TooltipWaitForHover {
    timer: Timer,
}

/// `Tooltip` that spawned nested from this one
#[derive(Debug, Component)]
#[relationship_target(relationship = TooltipsNestedOf)]
pub struct TooltipsNested(Entity);

/// This `Tooltip` is nested under this `Tooltip`
#[derive(Debug, Component)]
#[relationship(relationship_target = TooltipsNested)]
pub struct TooltipsNestedOf(Entity);

/// Place this on a node or text that you want to spawn a Tooltip.
/// The tooltip displayed will be the contents of `TooltipMap`
#[derive(Debug, Component)]
pub struct TooltipTermLink {
    linked_string: String,
}

impl TooltipTermLink {
    pub fn new(linked_string: impl ToString) -> Self {
        Self {
            linked_string: linked_string.to_string(),
        }
    }

    pub fn linked_string(&self) -> &str {
        &self.linked_string
    }
}

/// Timer added on creating a tooltip, if the user does not mouseover the tooltip in that
/// time then it will be despawned
#[derive(Debug, Component)]
pub struct TooltipLinkTimer {
    timer: Timer,
}

/// Sent when link has been hovered long enough to spawn `ToolTip`
#[derive(Event)]
struct TooltipLinkTimeElapsed {
    term_entity: Entity,
}

/// This is used for putting links of tooltips in tooltips
/// Should not be created by end users but can safely read if you are interested in recursive case
#[derive(Debug, Component)]
pub struct TooltipTermLinkRecursive {
    parent_entity: Entity,
    linked_string: String,
}

impl TooltipTermLinkRecursive {
    pub fn new(parent_entity: Entity, linked_string: String) -> Self {
        Self {
            parent_entity,
            linked_string,
        }
    }

    pub fn linked_string(&self) -> &str {
        &self.linked_string
    }

    pub fn parent_entity(&self) -> Entity {
        self.parent_entity
    }
}

/// The data of your tooltips.
/// When a `TooltipTermLink` is activated the string inside of it will be used as key
/// for the hashmap and its result will populate the tooltip
#[derive(Resource, Debug, Deref, DerefMut)]
pub struct TooltipMap {
    pub map: HashMap<String, Vec<TooltipsContent>>,
}

/// This makes up a part of the tooltips text content.
/// Each variant outputs text but with different behaviours
/// See each variants documenation for details
#[derive(Debug)]
pub enum TooltipsContent {
    /// Displays normal text for the user
    String(String),
    /// Nested information that can spawn's a child tooltip
    Term(String),
    /// Adds a highlight Component to all tooltips with `TooltipHighlight`
    Highlight(String),
}

/// Setup hooks so that interactions will work
/// I do not clean up the observers if the component is removed
/// I only anticipate this happening with despawn
fn setup_component_hooks(world: &mut World) {
    world
        .register_component_hooks::<TooltipTermLink>()
        .on_insert(|mut world, HookContext { entity, .. }| {
            world
                .commands()
                .entity(entity)
                .observe(middle_mouse_spawn)
                .observe(hover_time_spawn)
                .observe(hover_cancel_spawn);
        });

    world
        .register_component_hooks::<TooltipTermLinkRecursive>()
        .on_insert(|mut world, HookContext { entity, .. }| {
            world
                .commands()
                .entity(entity)
                .observe(middle_mouse_spawn)
                .observe(hover_time_spawn)
                .observe(hover_cancel_spawn);
        });

    world.register_component_hooks::<Tooltip>().on_insert(
        |mut world, HookContext { entity, .. }| {
            world
                .commands()
                .entity(entity)
                .observe(lock_tooltip)
                .observe(hover_debounce)
                .observe(hover_despawn);
        },
    );
}

#[derive(QueryData)]
#[query_data(mutable)]
struct HoverLinkQuery {
    link: AnyOf<(&'static TooltipTermLink, &'static TooltipTermLinkRecursive)>,
    timer: Option<&'static mut TooltipLinkTimer>,
}

/// This triggers for `ToolTip` links
/// If configured to display on hover this will add a timer that unless pointer moves
/// away from will spawn a `ToolTip`
fn hover_time_spawn(
    hover: On<TextHoveredOver>,
    tooltip_configuration: Res<TooltipConfiguration>,
    mut commands: Commands,
) {
    let current_activation = tooltip_configuration.activation_method.clone();
    if let ActivationMethod::Hover { time } = current_activation {
        {
            r!(commands.get_entity(hover.entity)).insert(TooltipLinkTimer {
                timer: Timer::new(time, TimerMode::Once),
            });
        }
    }
}

/// Removes hover timer when user's pointer has left
fn hover_cancel_spawn(hover: On<TextHoveredOut>, mut commands: Commands) {
    r!(commands.get_entity(hover.entity)).remove::<TooltipLinkTimer>();
}

#[derive(QueryData)]
#[query_data(mutable)]
struct SpawnLinksQuery {
    entity: Entity,
    link: AnyOf<(&'static TooltipTermLink, &'static TooltipTermLinkRecursive)>,
    spawn_timer: &'static mut TooltipLinkTimer,
}

#[derive(QueryData)]
#[query_data(mutable)]
struct HoverWaitQuery {
    entity: Entity,
    tooltip: &'static Tooltip,
    wait_for: &'static mut TooltipWaitForHover,
}

/// Tick timers and if they finish spawn/despawn the releveant tooltip
fn tick_timers(
    mut links_query: Query<SpawnLinksQuery>,
    mut wait_for_query: Query<HoverWaitQuery>,
    time_res: Res<Time>,
    hover: Option<Res<WasHoveringText>>,
    mut commands: Commands,
) {
    for mut links_item in &mut links_query {
        links_item.spawn_timer.timer.tick(time_res.delta());
        if links_item.spawn_timer.timer.is_finished() {
            commands.trigger(TooltipLinkTimeElapsed {
                term_entity: links_item.entity,
            });
            c!(commands.get_entity(links_item.entity)).remove::<TooltipLinkTimer>();
        }
    }
    for mut wait_for_item in &mut wait_for_query {
        match hover {
            Some(ref current) if current.entity == wait_for_item.tooltip.entity => {
                wait_for_item.wait_for.timer.reset();
            }
            _ => {
                wait_for_item.wait_for.timer.tick(time_res.delta());
                if wait_for_item.wait_for.timer.is_finished() {
                    c!(commands.get_entity(wait_for_item.entity)).try_despawn();
                }
            }
        }
    }
}

/// Triggered when timer is done, fetch additional data to spawn `ToolTip`
#[allow(clippy::too_many_arguments)]
fn spawn_time_done(
    term: On<TooltipLinkTimeElapsed>,
    links_query: Query<AnyOf<(&TooltipTermLink, &TooltipTermLinkRecursive)>>,
    existing_tooltips_query: Query<(Entity, &Tooltip)>,
    window_query: Query<&Window>,
    tooltips_map: Res<TooltipMap>,
    tooltip_reference: Res<TooltipReference>,
    tooltip_configuration: Res<TooltipConfiguration>,
    mut commands: Commands,
) {
    commands.remove_resource::<WasHoveringText>();
    spawn_tooltip(
        term.term_entity,
        links_query,
        existing_tooltips_query,
        window_query,
        tooltips_map,
        tooltip_reference,
        tooltip_configuration,
        &mut commands,
    );
}

#[derive(QueryData)]
struct TooltipDebounceQuery {
    tooltip: &'static Tooltip,
    debounced: Has<ToolTipDebounced>,
    cursor: &'static RelativeCursorPosition,
}

/// This is to debounce the cursor when it lands on the
/// tooltip, without this it is too easy to accidentally
/// close the tooltip
fn hover_debounce(
    hover: On<Pointer<Move>>,
    tooltip_query: Query<TooltipDebounceQuery>,
    mut commands: Commands,
) {
    const DEBOUNCE_DIST: f32 = 0.48;
    let tooltip_item = r!(tooltip_query.get(hover.entity));
    if tooltip_item.debounced {
        return;
    }
    let normalised = rq!(tooltip_item.cursor.normalized);
    let bounds = Rect {
        min: Vec2::new(-DEBOUNCE_DIST, -DEBOUNCE_DIST),
        max: Vec2::new(DEBOUNCE_DIST, DEBOUNCE_DIST),
    };

    if bounds.contains(normalised) {
        r!(commands.get_entity(hover.entity))
            .insert(ToolTipDebounced)
            .remove::<TooltipWaitForHover>();
    }
}

#[derive(QueryData)]
struct TooltipQuery {
    tooltip: &'static Tooltip,
    relative_cursor: &'static RelativeCursorPosition,
    has_nested: Has<TooltipsNested>,
    locked: Has<TooltipLocked>,
    debounced: Has<ToolTipDebounced>,
}

/// When user mouses out of `ToolTip` despawn it unless it has a nested tooltip
fn hover_despawn(
    hover: On<Pointer<Out>>,
    tooltip_query: Query<TooltipQuery>,
    mut commands: Commands,
) {
    let tooltip_item = r!(tooltip_query.get(hover.entity));

    // despawns occur at nested level
    if tooltip_item.has_nested || tooltip_item.locked || !tooltip_item.debounced {
        return;
    }

    if tooltip_item.relative_cursor.cursor_over {
        return;
    }
    r!(commands.get_entity(hover.entity)).despawn();
}

/// When user has pressed the middle mouse button on a `ToolTipLink`
#[allow(clippy::too_many_arguments)]
fn middle_mouse_spawn(
    press: On<TextMiddlePress>,
    links_query: Query<AnyOf<(&TooltipTermLink, &TooltipTermLinkRecursive)>>,
    existing_tooltips_query: Query<(Entity, &Tooltip)>,
    window_query: Query<&Window>,
    tooltips_map: Res<TooltipMap>,
    tooltip_reference: Res<TooltipReference>,
    tooltip_configuration: Res<TooltipConfiguration>,
    mut commands: Commands,
) {
    let current_activation = tooltip_configuration.activation_method.clone();
    if matches!(current_activation, ActivationMethod::MiddleMouse) {
        spawn_tooltip(
            press.entity,
            links_query,
            existing_tooltips_query,
            window_query,
            tooltips_map,
            tooltip_reference,
            tooltip_configuration,
            &mut commands,
        );
    }
}

/// Common logic to spawn `ToolTip` should be called when activation method has been satisfied
/// This also blocks tooltips from spawning if entity has already spawned one
#[allow(clippy::too_many_arguments)]
fn spawn_tooltip(
    term_entity: Entity,
    links_query: Query<'_, '_, AnyOf<(&TooltipTermLink, &TooltipTermLinkRecursive)>>,
    existing_tooltips_query: Query<(Entity, &Tooltip)>,
    window_query: Query<'_, '_, &Window>,
    tooltips_map: Res<'_, TooltipMap>,
    tooltip_reference: Res<'_, TooltipReference>,
    tooltip_configuration: Res<TooltipConfiguration>,
    commands: &mut Commands<'_, '_>,
) {
    // Prevent the same entity having two existing tooltips spawned
    for (_, tooltip) in existing_tooltips_query {
        if tooltip.entity == term_entity {
            return;
        }
    }

    let link_item = r!(links_query.get(term_entity));
    let (tooltip_term, nested) = match link_item {
        // Guranteed to have at least one entity
        (None, None) => {
            error!("Bevy invariant failed");
            return;
        }
        (None, Some(s)) => (s.linked_string.clone(), Some(s.parent_entity)),
        (Some(s), None) => (s.linked_string.clone(), None),
        // Shouldn't have both types of links could be caused by user if they tried hard enough
        (Some(_), Some(_)) => {
            error!("Nested tooltips has a bug");
            return;
        }
    };

    // Despawn other top level `ToolTip`s
    let zindex = match nested {
        None => {
            for (entity, _) in existing_tooltips_query {
                c!(commands.get_entity(entity)).try_despawn();
            }
            GlobalZIndex(tooltip_configuration.starting_z_index)
        }
        Some(_) => GlobalZIndex(
            existing_tooltips_query.count() as i32 + tooltip_configuration.starting_z_index,
        ),
    };

    let content = r!(tooltips_map.get(&tooltip_term));
    let design_node = position_tooltip(window_query, tooltip_reference);

    let mut tooltip_commands = commands.spawn((
        design_node,
        Tooltip {
            entity: term_entity,
        },
        TooltipWaitForHover {
            timer: Timer::new(
                tooltip_configuration.interaction_wait_for_time,
                TimerMode::Once,
            ),
        },
        zindex,
        Pickable {
            should_block_lower: true,
            is_hoverable: true,
        },
        children![(
            TooltipTitleNode,
            Node {
                display: Display::Flex,
                ..Default::default()
            },
            children![(TooltipTitleText, Text::new(tooltip_term))]
        )],
    ));
    if let Some(nested) = nested {
        tooltip_commands.insert(TooltipsNestedOf(nested));
    }
    tooltip_commands.with_children(|parent| {
        let parent_entity = parent.target_entity();
        parent
            .spawn((
                TooltipTextNode,
                Node {
                    display: Display::Flex,
                    width: Val::Percent(100.),
                    ..Default::default()
                },
                Text::new(""),
            ))
            .with_children(|text| {
                for c in content {
                    match c {
                        TooltipsContent::String(s) => {
                            text.spawn((TooltipStringText, TextSpan::new(s)));
                        }
                        TooltipsContent::Term(s) => {
                            text.spawn((
                                TooltipTermLinkRecursive::new(parent_entity, s.clone()),
                                TextSpan::new(s),
                            ));
                        }
                        TooltipsContent::Highlight(s) => {
                            text.spawn((TooltipHighlightLink(s.clone()), TextSpan::new(s)));
                        }
                    }
                }
            });
    });
    let tooltip_id = tooltip_commands.id();

    commands.trigger(TooltipSpawned { entity: tooltip_id });
}

/// Poistions the `ToolTip` relative to the cursor
fn position_tooltip(
    window_query: Query<'_, '_, &Window>,
    tooltip_reference: Res<'_, TooltipReference>,
) -> Node {
    let mut design_node = tooltip_reference.tooltip_node.clone();
    let window = r!(window_query.single());
    let cursor_position = r!(window.cursor_position());

    let window_size = window.size();
    let half_window_size = window_size / 2.0;
    let offset = 8.0;
    let (left, right) = if cursor_position.x > half_window_size.x {
        (
            Val::Auto,
            Val::Px(window_size.x - cursor_position.x + offset),
        )
    } else {
        (Val::Px(cursor_position.x + offset), Val::Auto)
    };
    let (top, bottom) = if cursor_position.y > half_window_size.y {
        (
            Val::Auto,
            Val::Px(window_size.y - cursor_position.y + offset),
        )
    } else {
        (Val::Px(cursor_position.y + offset), Val::Auto)
    };

    design_node.left = left;
    design_node.right = right;
    design_node.top = top;
    design_node.bottom = bottom;
    design_node
}

#[derive(QueryData)]
struct LockTooltipQuery {
    tooltip: &'static Tooltip,
    locked: Has<TooltipLocked>,
}

/// When user presses middle mouse button lock the ToolTip
fn lock_tooltip(
    press: On<Pointer<Press>>,
    tooltip_query: Query<LockTooltipQuery>,
    mut commands: Commands,
) {
    if press.button == PointerButton::Middle {
        let tooltip_item = r!(tooltip_query.get(press.entity));
        if tooltip_item.locked {
            r!(commands.get_entity(press.entity)).remove::<TooltipLocked>();
        } else {
            r!(commands.get_entity(press.entity)).insert(TooltipLocked);
        }
    }
}
