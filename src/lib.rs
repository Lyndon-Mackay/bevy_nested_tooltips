//! # Bevy Nested Tooltips
//!
//! ## Features
//! This library strives to handle the logic behind common tooltip features, while you focus on your unique data and design needs.
//!
//! - Tooltips can be spawned by hovering or by user pressing the middle mouse button, your choice which and you can change at runtime.
//! - Nesting to arbitrary levels, the only limitation is memory.
//! - Despawns if the user hasn't interacted with them in a configurable time period, or they mouse away after interacting with them.
//! - Locking by pressing of the middle mouse button. using observers you can implement your specific design to inform your users.
//! - Highlight other Entites using a linked text, highlight designs are up to you.
//!
//! ## Usage
//!
//! ### Import the prelude
//! ```rust
//! use bevy_nested_tooltips::prelude::*;
//! ```
//! ### Add the plugin
//!
//! ```rust
//!         .add_plugins((
//!             NestedTooltipPlugin,
//!         ))
//! ```
//!
//! ### (Optional) Configure tooltips
//! ```rust
//!     commands.insert_resource(TooltipConfiguration {
//!         activation_method: ActivationMethod::MiddleMouse,
//!         ..Default::default()
//!     });
//! ```
//!
//! ### Load your tooltips
//!
//! ```rust
//!     let mut tooltip_map = TooltipMap {
//!         map: HashMap::new(),
//!     };
//!
//!     tooltip_map.insert(
//!         "tooltip".into(),
//!         ToolTipsData::new(
//!             "ToolTip",
//!             vec![
//!                 TooltipsContent::String("A way to give users infomation can be ".into()),
//!                 TooltipsContent::Term("recursive".into()),
//!                 TooltipsContent::String(" Press middle mouse button to lock me. ".into()),
//!             ],
//!         ),
//!     );
//!
//!     tooltip_map.insert(
//!         "recursive".into(),
//!         ToolTipsData::new(
//!             "Recursive",
//!             vec![
//!                 TooltipsContent::String("Tooltips can be ".into()),
//!                 TooltipsContent::Term("recursive".into()),
//!                 TooltipsContent::String(
//!                     " You can highlight specific ui panels with such as the ".into(),
//!                 ),
//!                 TooltipsContent::Highlight("sides".into()),
//!                 TooltipsContent::String(" Press middle mouse button to lock me. ".into()),
//!             ],
//!         ),
//!     );
//! ```
//! ### Add links to relevant entities
//! ```rust
//! TooltipHighlight(vec!["sides".into()]),
//! ```
//! Or
//! ```rust
//!  TooltipTermLink::new("tooltip"),
//! ```
//!
//! ### Style your tooltips
//! Create an observer with at least these parameters.
//! ```rust
//! fn style_tooltip(
//!     new_tooltip: On<TooltipSpawned>,
//!     tooltip_info: TooltipEntitiesParam,
//!     mut commands: Commands,
//! )
//! ```
//! Fetch the data.
//! ```rust
//!     let tooltip_info = tooltip_info
//!         .tooltip_child_entities(new_tooltip.entity)
//!         .unwrap();
//! ```
//! Use the entities to style your node using commands or mutatable queries!
//! ```rust
//!     commands
//!         .get_entity(tooltip_info.title_node)
//!         .unwrap()
//!         .insert(Node {
//!             display: Display::Flex,
//!             justify_content: JustifyContent::Center,
//!             width: Val::Percent(100.),
//!             ..Default::default()
//!         });
//! ```
//!
//! #### React to changes.
//!
//! ```rust
//! // When highlighted change the colour, how you highlight is up to you
//! // maybe fancy animations
//! fn add_highlight(side: On<Add, TooltipHighlighting>, mut commands: Commands) {
//!     commands
//!         .get_entity(side.entity)
//!         .unwrap()
//!         .insert(BackgroundColor(GREEN.into()));
//! }
//!
//! // remove highlighting
//! fn remove_highlight(side: On<Remove, TooltipHighlighting>, mut commands: Commands) {
//!     commands
//!         .get_entity(side.entity)
//!         .unwrap()
//!         .insert(BackgroundColor(BLUE.into()));
//! }
//! ```

pub mod events;
pub mod highlight;
pub mod layout;
pub mod query;
pub mod term;

use std::time::Duration;

use bevy_app::{Plugin, PreStartup, Update};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    children,
    component::Component,
    entity::Entity,
    event::{EntityEvent, Event},
    lifecycle::HookContext,
    observer::{Observer, On},
    query::{AnyOf, Has, Or, QueryData, With},
    resource::Resource,
    schedule::{IntoScheduleConfigs, common_conditions::resource_changed},
    system::{Commands, Query, Res},
    world::World,
};

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

/// An easy way to import commonly used types.
pub mod prelude {
    pub use super::{
        ActivationMethod, NestedTooltipPlugin, Tooltip, TooltipConfiguration, TooltipMap,
        TooltipSpawned, TooltipsContent, TooltipsData,
        events::{TooltipHighlighting, TooltipLocked},
        highlight::{TooltipHighlight, TooltipHighlightLink},
        layout::{TooltipStringText, TooltipTextNode, TooltipTitleNode, TooltipTitleText},
        query::{TooltipEntities, TooltipEntitiesParam},
        term::{TooltipTermLink, TooltipTermLinkRecursive},
    };
}
use prelude::*;

use crate::{highlight::HighlightPlugin, term::hover_time_spawn};

/// This plugin adds systems and resources that makes the logic work.
pub struct NestedTooltipPlugin;

impl Plugin for NestedTooltipPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_plugins(HighlightPlugin)
            .init_resource::<TooltipConfiguration>()
            .init_resource::<TooltipReference>()
            .add_systems(PreStartup, setup_component_hooks)
            .add_systems(Update, tick_timers)
            .add_systems(
                Update,
                update_settings.run_if(resource_changed::<TooltipConfiguration>),
            )
            .add_observer(spawn_time_done);
    }
}

/// Resource that configures the behaviour of tooltips.
#[derive(Resource, Debug)]
pub struct TooltipConfiguration {
    /// See the [`ActivationMethod`] variants.
    pub activation_method: ActivationMethod,

    /// Maximum amount of time the `ToolTip` will remain around without user interaction.
    pub interaction_wait_for_time: Duration,

    /// The starting z_index this will be incremented for each recursive tooltip
    /// increase this if tooltips are not on top and you want to fix that.
    pub starting_z_index: i32,
}

impl Default for TooltipConfiguration {
    fn default() -> Self {
        Self {
            activation_method: Default::default(),
            interaction_wait_for_time: Duration::from_secs_f64(0.8),
            starting_z_index: 3,
        }
    }
}

/// How a tooltip is triggered by default this is done via hovering
/// Hovering can be further customised.
#[derive(Debug, Clone)]
pub enum ActivationMethod {
    /// Middle mouse button is pressed.
    MiddleMouse,
    /// Mouse is over the `Tooltip` for a duration.
    Hover { time: Duration },
}

impl Default for ActivationMethod {
    fn default() -> Self {
        ActivationMethod::Hover {
            time: Duration::from_secs_f64(0.9),
        }
    }
}

/// Default node for the [`Tooltip`] node use this to layout your tooltips without
/// accidentally moving it's position.
/// This resource is initialised on adding plugin.
#[derive(Resource, Debug)]
pub struct TooltipReference {
    /// Top level Node this will be copied to the [`Tooltip`] positions will be overwritten
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
/// until this one is finished to prevent tooltip jumping around.
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
/// leaving will now despawn this tooltip.
#[derive(Debug, Component)]
struct ToolTipDebounced;

/// This is sent when a [`Tooltip`] is spawned.
#[derive(Debug, EntityEvent)]
pub struct TooltipSpawned {
    pub entity: Entity,
}

/// If the user hasn't hovered on the tooltip in the specified time despawn it
/// time is configured in [`TooltipConfiguration`].
#[derive(Debug, Component)]
pub struct TooltipWaitForHover {
    timer: Timer,
}

/// [`Tooltip`] that spawned nested from this one.
#[derive(Debug, Component)]
#[relationship_target(relationship = TooltipsNestedOf)]
pub struct TooltipsNested(Entity);

/// This [`Tooltip`] is nested under the entities [`Tooltip`].
#[derive(Debug, Component)]
#[relationship(relationship_target = TooltipsNested)]
pub struct TooltipsNestedOf(Entity);

/// Timer added on creating a [`Tooltip`], if the user does not mouseover the tooltip in that
/// time then it will be despawned.
#[derive(Debug, Component)]
pub struct TooltipLinkTimer {
    timer: Timer,
}

/// Sent when link has been hovered long enough to spawn [`ToolTip`].
#[derive(Event)]
struct TooltipLinkTimeElapsed {
    term_entity: Entity,
}

/// The data of your tooltips.
/// When a [`TooltipTermLink`] is activated the string inside of it will be used as key
/// for the hashmap and its result will populate the tooltip.
///
/// See [`TooltipsData`].
#[derive(Resource, Debug, Deref, DerefMut, Clone)]
pub struct TooltipMap {
    pub map: HashMap<String, TooltipsData>,
}

/// What is to be included in the [`Tooltip`].
#[derive(Debug, Clone)]
pub struct TooltipsData {
    /// The title at the top of the tooltips.
    pub title: String,
    /// The rest of the text.
    pub content: Vec<TooltipsContent>,
}

impl TooltipsData {
    pub fn new(title: impl ToString, content: Vec<TooltipsContent>) -> Self {
        Self {
            title: title.to_string(),
            content,
        }
    }
}

/// This makes up a part of the tooltips text content.
/// Each variant outputs text but with different behaviours
/// See each variants documenation for details.
#[derive(Debug, Clone)]
pub enum TooltipsContent {
    /// Displays normal text for the user.
    String(String),
    /// Nested information that can spawn's a child tooltip, used as key for [`TooltipMap`].
    Term(String),
    /// Adds a highlight Component to all tooltips with [`TooltipHighlight`].
    Highlight(String),
}

/// Marker for Observers related to middle mouse triggering of tooltips
#[derive(Component)]
struct NestedTooltipsMiddleMouseObserver;

/// Marker for Observers related to hover triggering of tooltips
#[derive(Component)]
struct NestedTooltipsHoverObserver;

/// Setup hooks so that interactions will work
/// This is based on resource setting
/// If setting is changed then an update system will set the correct observers
fn setup_component_hooks(world: &mut World) {
    world
        .register_component_hooks::<TooltipTermLink>()
        .on_insert(|mut world, HookContext { entity, .. }| {
            let config = rq!(world.get_resource::<TooltipConfiguration>());

            match config.activation_method {
                ActivationMethod::MiddleMouse => {
                    let middle_observe = Observer::new(middle_mouse_spawn).with_entity(entity);
                    world
                        .commands()
                        .spawn((middle_observe, NestedTooltipsMiddleMouseObserver));
                }
                ActivationMethod::Hover { .. } => {
                    let hover_spawn_observer = Observer::new(hover_time_spawn).with_entity(entity);
                    let hover_cancel_observer =
                        Observer::new(hover_cancel_spawn).with_entity(entity);

                    world
                        .commands()
                        .spawn((hover_spawn_observer, NestedTooltipsHoverObserver));
                    world
                        .commands()
                        .spawn((hover_cancel_observer, NestedTooltipsHoverObserver));
                }
            }
        });

    world
        .register_component_hooks::<TooltipTermLinkRecursive>()
        .on_insert(|mut world, HookContext { entity, .. }| {
            let config = rq!(world.get_resource::<TooltipConfiguration>());

            match config.activation_method {
                ActivationMethod::MiddleMouse => {
                    let middle_observe = Observer::new(middle_mouse_spawn).with_entity(entity);
                    world
                        .commands()
                        .spawn((middle_observe, NestedTooltipsMiddleMouseObserver));
                }
                ActivationMethod::Hover { .. } => {
                    let hover_spawn_observer = Observer::new(hover_time_spawn).with_entity(entity);
                    let hover_cancel_observer =
                        Observer::new(hover_cancel_spawn).with_entity(entity);

                    world
                        .commands()
                        .spawn((hover_spawn_observer, NestedTooltipsHoverObserver));
                    world
                        .commands()
                        .spawn((hover_cancel_observer, NestedTooltipsHoverObserver));
                }
            }
        });

    world.register_component_hooks::<Tooltip>().on_insert(
        |mut world, HookContext { entity, .. }| {
            world
                .commands()
                .entity(entity)
                .observe(toggle_lock)
                .observe(hover_debounce)
                .observe(hover_despawn);
        },
    );
}

/// Updates the observers to match user settings
/// this will despawn unused observers
#[allow(clippy::type_complexity)]
fn update_settings(
    config: Res<TooltipConfiguration>,
    term_links: Query<Entity, Or<(With<TooltipTermLink>, With<TooltipTermLinkRecursive>)>>,
    mut commands: Commands,
) {
    match config.activation_method {
        ActivationMethod::MiddleMouse => {
            let mut middle_observe = Observer::new(middle_mouse_spawn);
            for entity in term_links {
                middle_observe.watch_entity(entity);
            }
            commands.spawn((middle_observe, NestedTooltipsMiddleMouseObserver));
        }
        ActivationMethod::Hover { .. } => {
            let mut hover_spawn_observer = Observer::new(hover_time_spawn);
            let mut hover_cancel_observer = Observer::new(hover_cancel_spawn);

            for entity in term_links {
                hover_spawn_observer.watch_entity(entity);
                hover_cancel_observer.watch_entity(entity);
            }
            commands.spawn((hover_spawn_observer, NestedTooltipsHoverObserver));
            commands.spawn((hover_cancel_observer, NestedTooltipsHoverObserver));
        }
    }
}

#[derive(QueryData)]
#[query_data(mutable)]
struct HoverLinkQuery {
    link: AnyOf<(&'static TooltipTermLink, &'static TooltipTermLinkRecursive)>,
    timer: Option<&'static mut TooltipLinkTimer>,
}

/// Removes hover timer when user's pointer has left.
fn hover_cancel_spawn(hover: On<Pointer<Out>>, mut commands: Commands) {
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

/// Tick timers and if they finish spawn/despawn the releveant tooltip.
fn tick_timers(
    mut links_query: Query<SpawnLinksQuery>,
    mut wait_for_query: Query<HoverWaitQuery>,
    time_res: Res<Time>,
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
        wait_for_item.wait_for.timer.tick(time_res.delta());
        if wait_for_item.wait_for.timer.is_finished() {
            c!(commands.get_entity(wait_for_item.entity)).try_despawn();
        }
    }
}

/// Triggered when timer is done, fetch additional data to spawn [`ToolTip`].
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
/// close the tooltip.
fn hover_debounce(
    hover: On<Pointer<Move>>,
    tooltip_query: Query<TooltipDebounceQuery>,
    mut commands: Commands,
) {
    // Number should not be greater then 0.5
    // the low the number the more in the tooltip, the pointer needs to be
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

/// When user mouses out of [`ToolTip`] despawn it unless it has a nested tooltip.
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

/// When user has pressed the middle mouse button on a [`TooltipLink`].
#[allow(clippy::too_many_arguments)]
fn middle_mouse_spawn(
    mut press: On<Pointer<Press>>,
    links_query: Query<AnyOf<(&TooltipTermLink, &TooltipTermLinkRecursive)>>,
    existing_tooltips_query: Query<(Entity, &Tooltip)>,
    window_query: Query<&Window>,
    tooltips_map: Res<TooltipMap>,
    tooltip_reference: Res<TooltipReference>,
    tooltip_configuration: Res<TooltipConfiguration>,
    mut commands: Commands,
) {
    // Stop tooltip lock being triggered
    press.propagate(false);
    if press.button != PointerButton::Middle {
        return;
    }
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

/// Common logic to spawn [`ToolTip`] should be called when activation method has been satisfied
/// This also blocks tooltips from spawning if entity has already spawned one.
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

    let tooltip_data = r!(tooltips_map.get(&tooltip_term));
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
            children![(TooltipTitleText, Text::new(tooltip_data.title.clone()))]
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
                for c in &tooltip_data.content {
                    match c.clone() {
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

/// Poistions the [`ToolTip`] relative to the cursor.
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

/// When user presses middle mouse button add or remove [`TooltipLocked`].
fn toggle_lock(
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
