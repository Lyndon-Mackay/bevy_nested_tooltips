//! `TextSpan`'s do not currently support observers so this file is here to read hovers on text
//! and to narrow it down to the actual textspan.

use bevy_ecs::{
    entity::Entity,
    event::EntityEvent,
    query::{AnyOf, Or, With, Without},
    resource::Resource,
    system::{Commands, Query, Res},
};
use bevy_log::info;
use bevy_math::Rect;
use bevy_text::TextLayoutInfo;
use bevy_ui::UiGlobalTransform;
use bevy_window::Window;
use tiny_bail::prelude::*;

use crate::{
    TooltipHighlightLink, TooltipTermLink, TooltipTermLinkRecursive, TooltipTextNode,
    TooltipsNested,
};

/// Used to track for hovering when resource is present mouse was located
/// on the rect last frame
#[derive(Resource, Clone, Copy)]
pub(crate) struct WasHoveringText {
    pub(crate) entity: Entity,
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

/// Check with the topmost tooltip and see if any text is hovered
#[allow(clippy::type_complexity)]
pub(crate) fn hover_text_span(
    // We look in toop most tooltip first
    layout_info_query: Query<&TextLayoutInfo, (With<TooltipTextNode>, Without<TooltipsNested>)>,
    //If we don't find anything in top most tooltip we search top level link
    non_tooltip_info_query: Query<
        (&TextLayoutInfo, &UiGlobalTransform),
        (
            Without<TooltipTextNode>,
            Or<(With<TooltipTermLink>, With<TooltipHighlightLink>)>,
        ),
    >,
    activated_text_query: Query<
        AnyOf<(
            &TooltipTermLink,
            &TooltipTermLinkRecursive,
            &TooltipHighlightLink,
        )>,
    >,
    windows_query: Query<&Window>,
    was_hovering: Option<Res<WasHoveringText>>,
    mut commands: Commands,
) {
    let window = r!(windows_query.single());
    let cursor = rq!(window.cursor_position());

    //If we were hovering a text section then check if we still are
    if let Some(hovered) = was_hovering {
        if let Ok(text_layout) = layout_info_query.get(hovered.entity)
            && let Some(rect) = text_layout
                .section_rects
                .iter()
                .find(|s| s.0 == hovered.entity)
            && rect.1.contains(cursor)
        {
            return;
            // Do nothing we still hovering
        } else {
            commands.remove_resource::<WasHoveringText>();
            commands.trigger(TextHoveredOut {
                entity: hovered.entity,
            });

            // Hovered out now lets see if we hovered into anything else
        }
    }

    // check if we hovered on text section
    for text_layout in layout_info_query {
        let hovering = cq!(text_layout
            .section_rects
            .iter()
            .find(|x| x.1.contains(cursor)))
        .0;
        if activated_text_query.contains(hovering) {
            commands.trigger(TextHoveredOver { entity: hovering });
            commands.insert_resource(WasHoveringText { entity: hovering });
            // match selected_text_spans {
            //     (None, None, None) => {
            //         error!("Bevy invariant not upheld");
            //         return;
            //     }
            //     (None, None, Some(_)) => {
            //         commands.trigger(TextHoveredOver { entity: hovering });
            //         commands.insert_resource(WasHoveringText { entity: hovering });
            //         return;
            //     }
            //     (None, Some(_), None) => {
            //         commands.trigger(TextHoveredOver { entity: hovering });
            //         commands.insert_resource(WasHoveringText { entity: hovering });
            //         return;
            //     }
            //     (Some(_), None, None) => {
            //         commands.trigger(TextHoveredOver { entity: hovering });
            //         commands.insert_resource(WasHoveringText { entity: hovering });
            //         return;
            //     }

            //     a => {
            //         error!("Highlight and term at the same time not support {a:?}");
            //         return;
            //     }
            // }
        }
    }

    for (text_layout, uiglobal) in non_tooltip_info_query {
        // bevy_log::info!("try");

        bevy_log::info!("{:?}", cursor);
        bevy_log::info!("{:?}", text_layout.section_rects);
        // bevy_log::info!("{:?}", text_layout.glyphs);
        // bevy_log::info!("{:?}", transform.translation);

        // let cursor = cursor * (1. / text_layout.scale_factor);
        let hovering = cq!(text_layout.section_rects.iter().find(|x| {
            let min = uiglobal.transform_point2(x.1.min);
            let max = uiglobal.transform_point2(x.1.max);
            Rect { min, max }.contains(cursor)
        }))
        .0;
        // let hovering = cq!(text_layout
        //     .section_rects
        //     .iter()
        //     .find(|x| x.1.contains(cursor)))
        // .0;

        if activated_text_query.contains(hovering) {
            info!("got it");
            commands.trigger(TextHoveredOver { entity: hovering });
            commands.insert_resource(WasHoveringText { entity: hovering });
        }
    }
}
