use bevy::prelude::*;
use bevy_color::palettes::css::{BLUE, GREEN, ORANGE, ORANGE_RED, WHITE, YELLOW_GREEN};
use bevy_inspector_egui::{bevy_egui::EguiPlugin, quick::WorldInspectorPlugin};
use bevy_nested_tooltips::{
    NestedTooltipPlugin, Tooltip, TooltipMap, TooltipSpawned, TooltipsContent,
    events::{TooltipHighlighting, TooltipLocked},
    highlight::{TooltipHighlight, TooltipHighlightLink},
    layout::{TooltipTitleNode, TooltipTitleText},
    term::{TooltipTermLink, TooltipTermLinkRecursive},
};
use bevy_platform::collections::HashMap;
use bevy_ui::RelativeCursorPosition;
use bevy_window::WindowMode;

#[derive(Component)]
struct LockMessage;

fn main() -> AppExit {
    App::new()
        .add_plugins((
            //This library only works for fullscreen
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    mode: WindowMode::BorderlessFullscreen(MonitorSelection::Current),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            NestedTooltipPlugin,
        ))
        .add_plugins(EguiPlugin::default())
        .add_plugins(WorldInspectorPlugin::new())
        .add_systems(Startup, spawn_scene)
        // Note having that many observers is not neccessary
        // just it's clearer what each example does
        // Also easier to prototype
        .add_observer(style_tooltip)
        .add_observer(center_title)
        .add_observer(title_font)
        .add_observer(term_font)
        // Look at query style to get this done without using so many observers
        .add_observer(query_style)
        // These observers are more necessary to react to user
        .add_observer(add_highlight)
        .add_observer(remove_highlight)
        .add_observer(display_locking)
        .add_observer(display_unlocking)
        .run()
}

fn spawn_scene(mut commands: Commands) {
    commands.spawn(Camera2d);

    edge_panels(&mut commands);
    let interaction_screen = Node {
        left: Val::Percent(30.),
        top: Val::Percent(30.),
        width: Val::Vw(45.),
        height: Val::Vh(45.),
        display: Display::Grid,
        grid_template_rows: vec![GridTrack::fr(1.), GridTrack::fr(5.)],
        position_type: PositionType::Absolute,
        ..Default::default()
    };

    let background_colour = BackgroundColor(Oklcha::lch(0.7, 0.1, 229.).into());
    commands.spawn((
        interaction_screen,
        background_colour,
        children![
            (
                Node {
                    display: Display::Flex,
                    justify_content: JustifyContent::Center,
                    width: Val::Percent(100.),
                    ..Default::default()
                },
                children![(
                    Text::new("Bevy nested tooltips!"),
                    TextFont {
                        font_size: 50.,
                        ..Default::default()
                    }
                )]
            ),
            (
                Node {
                    width: Val::Percent(100.),
                    ..Default::default()
                },
                BackgroundColor(YELLOW_GREEN.into()),
                Text::new("I am a "),
                RelativeCursorPosition::default(),
                children![
                    (
                        TextSpan::new("ToolTip"),
                        TooltipTermLink::new("tooltip"),
                        TextColor(BLUE.into())
                    ),
                    TextSpan::new(" hover over it! "),
                    (
                        TextSpan::new("top"),
                        TooltipHighlightLink("top".into()),
                        TextColor(GREEN.into())
                    ),
                    TextSpan::new(" "),
                    (
                        TextSpan::new("bottom"),
                        TooltipHighlightLink("bottom".into()),
                        TextColor(GREEN.into())
                    ),
                ]
            )
        ],
    ));

    let mut tooltip_map = TooltipMap {
        map: HashMap::new(),
    };

    tooltip_map.insert(
        "tooltip".into(),
        vec![
            TooltipsContent::String("A way to give users infomation can be ".into()),
            TooltipsContent::Term("recursive".into()),
            TooltipsContent::String(" Press middle mouse button to lock me. ".into()),
        ],
    );

    tooltip_map.insert(
        "recursive".into(),
        vec![
            TooltipsContent::String("Tooltips can be ".into()),
            TooltipsContent::Term("recursive".into()),
            TooltipsContent::String(
                " You can highlight specific ui panels with such as the ".into(),
            ),
            TooltipsContent::Highlight("sides".into()),
            TooltipsContent::String(" Press middle mouse button to lock me. ".into()),
        ],
    );

    commands.insert_resource(tooltip_map);
}

fn edge_panels(commands: &mut Commands) {
    let left_node = Node {
        position_type: PositionType::Absolute,
        left: percent(0),
        top: percent(10),
        bottom: auto(),
        width: percent(5),
        height: percent(80),
        ..Default::default()
    };
    commands.spawn((
        left_node,
        BackgroundColor(BLUE.into()),
        TooltipHighlight("sides".into()),
    ));
    let right_node = Node {
        position_type: PositionType::Absolute,
        right: percent(0),
        top: percent(10),
        width: percent(5),
        height: percent(80),
        ..Default::default()
    };
    commands.spawn((
        right_node,
        BackgroundColor(BLUE.into()),
        TooltipHighlight("sides".into()),
    ));

    let top_node = Node {
        position_type: PositionType::Absolute,
        right: percent(10),
        top: percent(0),
        width: percent(80),
        height: percent(10),
        ..Default::default()
    };

    commands.spawn((
        top_node,
        BackgroundColor(BLUE.into()),
        TooltipHighlight("top".into()),
    ));

    let bottom_node = Node {
        position_type: PositionType::Absolute,
        right: percent(10),
        bottom: percent(0),
        width: percent(80),
        height: percent(10),
        ..Default::default()
    };
    commands.spawn((
        bottom_node,
        BackgroundColor(BLUE.into()),
        TooltipHighlight("bottom".into()),
    ));
}

// This is how you style a tooltip!
// If you want to change the default node consider using TooltipReference
fn style_tooltip(tooltip: On<Add, Tooltip>, mut commands: Commands) {
    commands
        .get_entity(tooltip.entity)
        .unwrap()
        .insert((BackgroundColor(ORANGE.into()), BorderColor::all(WHITE)));
}

// Style and center the title here
fn center_title(title_node: On<Add, TooltipTitleNode>, mut commands: Commands) {
    commands
        .get_entity(title_node.entity)
        .unwrap()
        .insert(Node {
            display: Display::Flex,
            justify_content: JustifyContent::Center,
            width: Val::Percent(100.),
            ..Default::default()
        });
}

fn title_font(title_text: On<Add, TooltipTitleText>, mut commands: Commands) {
    commands
        .get_entity(title_text.entity)
        .unwrap()
        .insert(TextFont {
            font_size: 40.,
            ..Default::default()
        });
}

fn term_font(
    term_text: On<Add, (TooltipTermLink, TooltipTermLinkRecursive)>,
    mut commands: Commands,
) {
    commands
        .get_entity(term_text.entity)
        .unwrap()
        .insert(TextColor(BLUE.into()));
}

// If you prefer you can listen to this observer and style via querying
// Static styling can be done entirely here
fn query_style(
    new_tooltip: On<TooltipSpawned>,
    ancestor_query: Query<&ChildOf>,
    highlight_query: Query<Entity, With<TooltipHighlightLink>>,
    mut commands: Commands,
) {
    for current_highlight in highlight_query {
        if new_tooltip.entity == ancestor_query.root_ancestor(current_highlight) {
            commands
                .get_entity(current_highlight)
                .unwrap()
                .insert(TextColor(GREEN.into()));
            return;
        }
    }
}

// When highlighted change the colour, how you highlight is up to you
// maybe fancy animations
fn add_highlight(side: On<Add, TooltipHighlighting>, mut commands: Commands) {
    // info!("style");
    commands
        .get_entity(side.entity)
        .unwrap()
        .insert(BackgroundColor(GREEN.into()));
}

// remove highlighting
fn remove_highlight(side: On<Remove, TooltipHighlighting>, mut commands: Commands) {
    // info!("style");
    commands
        .get_entity(side.entity)
        .unwrap()
        .insert(BackgroundColor(BLUE.into()));
}

fn display_locking(lock: On<Add, TooltipLocked>, mut commands: Commands) {
    // Making this actually look nice is an excercise for the reader
    let id = commands
        .spawn((
            Text::new("I have been locked"),
            TextFont::from_font_size(10.),
            LockMessage,
        ))
        .id();
    commands
        .get_entity(lock.entity)
        .unwrap()
        .insert(BackgroundColor(ORANGE_RED.into()))
        .add_child(id);
}

fn display_unlocking(
    lock: On<Remove, TooltipLocked>,
    message_lock_query: Query<(Entity, &ChildOf), With<LockMessage>>,
    mut commands: Commands,
) {
    commands
        .get_entity(lock.entity)
        .unwrap()
        .insert(BackgroundColor(ORANGE.into()));
    if let Some((entity, _)) = message_lock_query
        .iter()
        .find(|item| item.1.0 == lock.entity)
    {
        commands.get_entity(entity).unwrap().despawn();
    }
}
