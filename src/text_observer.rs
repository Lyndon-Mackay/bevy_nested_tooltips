//! `TextSpan`'s do not currently support observers so this file is here to read hovers on text
//! and to narrow it down to the actual textspan.

use bevy_app::{Plugin, Update};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    event::EntityEvent,
    hierarchy::ChildOf,
    lifecycle::Add,
    observer::On,
    query::{Or, QueryData, With, Without},
    resource::Resource,
    system::{Commands, Query, Res},
};
use bevy_text::TextLayoutInfo;
use bevy_ui::{ComputedNode, RelativeCursorPosition, widget::Text};
use tiny_bail::prelude::*;

use crate::{TooltipHighlightLink, TooltipTermLink, TooltipTermLinkRecursive, TooltipsNested};

/// Plugin to bridge gap until text spans support observers
pub(crate) struct TextObservePlugin;

impl Plugin for TextObservePlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(Update, tooltip_links)
            .add_observer(term_link_textspan_parent)
            .add_observer(recursive_term_link_textspan_parent)
            .add_observer(highlight_link_textspan_parent);
    }
}

/// Used to track for hovering when resource is present mouse was located
/// on the rect last frame
#[derive(Resource, Clone, Copy)]
pub(crate) struct WasHoveringText {
    /// Actual entity that we hovered
    pub(crate) entity: Entity,
    /// Entity which holds relative cursor this will be
    /// different from actual hovered entity in the case
    /// of text spans
    pub(crate) relative_cursor_entity: Entity,
}

/// Term has been hovered in the tooltip
#[derive(Debug, EntityEvent)]
pub(crate) struct TextHoveredOver {
    pub(crate) entity: Entity,
}

/// Term has been hovered out the tooltip
#[derive(Debug, EntityEvent)]
pub(crate) struct TextHoveredOut {
    pub(crate) entity: Entity,
}

/// This is to mark text as having a textspan that contains a link
/// RelativeCursorPosition and observers do not work with textspan
/// So will listen to parent instead and check the span
///
#[derive(Component, Debug)]
#[require(RelativeCursorPosition)]
pub(crate) struct ToolTipListenTextSpan;

pub(crate) fn highlight_link_textspan_parent(
    add: On<Add, TooltipHighlightLink>,
    text_query: Query<Entity, With<Text>>,
    ancestor_query: Query<&ChildOf>,
    mut commands: Commands,
) {
    // Can already listen to text so no need to do anything else
    if text_query.contains(add.entity) {
        r!(commands.get_entity(add.entity)).insert(RelativeCursorPosition::default());
        return;
    }
    for entity in ancestor_query.iter_ancestors(add.entity) {
        if text_query.contains(entity) {
            r!(commands.get_entity(entity)).insert(ToolTipListenTextSpan);
            return;
        }
    }
}

pub(crate) fn term_link_textspan_parent(
    add: On<Add, TooltipTermLink>,
    text_query: Query<Entity, With<Text>>,
    ancestor_query: Query<&ChildOf>,
    mut commands: Commands,
) {
    // Can already listen to text so no need to do anything else
    if text_query.contains(add.entity) {
        r!(commands.get_entity(add.entity)).insert(RelativeCursorPosition::default());
        return;
    }
    for entity in ancestor_query.iter_ancestors(add.entity) {
        if text_query.contains(entity) {
            r!(commands.get_entity(entity)).insert(ToolTipListenTextSpan);
            return;
        }
    }
}

pub(crate) fn recursive_term_link_textspan_parent(
    add: On<Add, TooltipTermLinkRecursive>,
    text_query: Query<Entity, With<Text>>,
    ancestor_query: Query<&ChildOf>,
    mut commands: Commands,
) {
    // Can already listen to text so no need to do anything else
    if text_query.contains(add.entity) {
        r!(commands.get_entity(add.entity)).insert(RelativeCursorPosition::default());
        return;
    }
    for entity in ancestor_query.iter_ancestors(add.entity) {
        if text_query.contains(entity) {
            r!(commands.get_entity(entity)).insert(ToolTipListenTextSpan);
            return;
        }
    }
}

#[derive(QueryData)]
struct TooltipLinksQuery {
    entity: Entity,
    text_layout_info: &'static TextLayoutInfo,
    compute_node: &'static ComputedNode,
    relative_cursor: &'static RelativeCursorPosition,
}

/// Check with the topmost tooltip and see if any text is hovered
#[allow(clippy::type_complexity)]
fn tooltip_links(
    //If we don't find anything in top most tooltip we search top level link
    tooltip_links_query: Query<
        TooltipLinksQuery,
        (
            Without<TooltipsNested>,
            Or<(
                With<TooltipTermLink>,
                With<TooltipHighlightLink>,
                With<ToolTipListenTextSpan>,
            )>,
        ),
    >,
    was_hovering: Option<Res<WasHoveringText>>,
    mut commands: Commands,
) {
    //If we were hovering a text section then check if we still are
    if let Some(hovered) = was_hovering {
        let links_item = match tooltip_links_query.get(hovered.relative_cursor_entity) {
            Ok(item) => item,
            Err(_) => {
                commands.remove_resource::<WasHoveringText>();
                return;
            }
        };
        let relative = links_item.relative_cursor;
        let ui_node = links_item.compute_node;
        let text_layout = links_item.text_layout_info;

        match relative.normalized {
            Some(norm) => {
                let adjusted_cursor_position = ui_node.size() / 2. + norm * ui_node.size();
                if let Some(rect) = text_layout
                    .section_rects
                    .iter()
                    .find(|rect| rect.1.contains(adjusted_cursor_position))
                {
                    if rect.0 != hovered.entity {
                        commands.remove_resource::<WasHoveringText>();
                        commands.trigger(TextHoveredOut {
                            entity: hovered.entity,
                        });
                    }
                    return;
                }
            }
            None => {
                commands.remove_resource::<WasHoveringText>();
                commands.trigger(TextHoveredOut {
                    entity: hovered.entity,
                });
                return;
            }
        }
    }

    for links_item in tooltip_links_query {
        let entity = links_item.entity;
        let relative = links_item.relative_cursor;
        let ui_node = links_item.compute_node;
        let text_layout = links_item.text_layout_info;
        if relative.cursor_over
            && let Some(norm) = relative.normalized
        {
            let adjusted_cursor_position = ui_node.size() / 2. + norm * ui_node.size();

            if let Some((hovered_entity, _)) = text_layout
                .section_rects
                .iter()
                .find(|rect| rect.1.contains(adjusted_cursor_position))
                .copied()
            {
                commands.trigger(TextHoveredOver {
                    entity: hovered_entity,
                });
                commands.insert_resource(WasHoveringText {
                    entity: hovered_entity,
                    relative_cursor_entity: entity,
                });
                return;
            }
        }
    }
}
