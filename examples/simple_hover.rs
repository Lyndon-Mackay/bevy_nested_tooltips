use bevy::prelude::*;
use bevy_color::palettes::css::{BLUE, GREEN, ORANGE, WHITE, YELLOW_GREEN};
use bevy_inspector_egui::{bevy_egui::EguiPlugin, quick::WorldInspectorPlugin};
use bevy_nested_tooltips::{
    NestedTooltipPlugin, Tooltip, TooltipHighlightText, TooltipMap, TooltipSpawned,
    TooltipTermLink, TooltipTermText, TooltipTitleNode, TooltipTitleText, TooltipsContent,
};
use bevy_platform::collections::HashMap;
use bevy_ui::RelativeCursorPosition;

fn main() -> AppExit {
    App::new()
        .add_plugins((DefaultPlugins, NestedTooltipPlugin))
        .add_plugins(EguiPlugin::default())
        .add_plugins(WorldInspectorPlugin::new())
        .add_systems(Startup, spawn_scene)
        .add_observer(style_tooltip)
        .add_observer(center_title)
        .add_observer(title_font)
        .add_observer(term_font)
        .add_observer(query_style)
        .run()
}

fn spawn_scene(mut commands: Commands) {
    commands.spawn(Camera2d);

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
                children![(
                    Text::new("I am a "),
                    RelativeCursorPosition::default(),
                    children![
                        (
                            TextSpan::new("ToolTip"),
                            TooltipTermLink::new("tooltip"),
                            TextColor(BLUE.into())
                        ),
                        TextSpan::new(" hover over it!")
                    ]
                )]
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
        ],
    );

    commands.insert_resource(tooltip_map);
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

fn term_font(term_text: On<Add, TooltipTermText>, mut commands: Commands) {
    commands
        .get_entity(term_text.entity)
        .unwrap()
        .insert(TextColor(BLUE.into()));
}

// If you prefer you can listen to this observer and style via querying
fn query_style(
    new_tooltip: On<TooltipSpawned>,
    ancestor_query: Query<&ChildOf>,
    highlight_query: Query<Entity, With<TooltipHighlightText>>,
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
