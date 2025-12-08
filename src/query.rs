//! Contains the convenienece queries and systemparams to easily get the
//! entities for each part of a single [`Tooltip`].

use bevy_ecs::{
    entity::Entity,
    hierarchy::ChildOf,
    query::With,
    system::{Query, SystemParam},
};
use tiny_bail::prelude::*;

use crate::{
    layout::{TooltipStringText, TooltipTextNode, TooltipTitleNode, TooltipTitleText},
    prelude::TooltipHighlightLink,
    term::TooltipTermLinkRecursive,
};

/// For a [`Tooltip`] these are descendent parts that make up it.
/// This assumes and does not check that the tooltip is in good order.
pub struct TooltipEntites {
    /// The entity of the node holding the title text.
    /// [`TooltipTitleNode`]
    /// Tooltip should not have more then one.
    pub title_node: Entity,
    /// The Entity of the title_text.
    /// That has [`TooltipTitleText`].
    /// Tooltip should not have more then one.
    pub title_text: Entity,

    /// The entity of the node holding the combined info of all the non-title text.
    /// That has [`TooltipTextNode`].
    /// Tooltip should not have more then one.
    pub tooltip_text_node: Entity,

    /// All entities of all plain texts with no effects.
    /// That is [`TooltipStringText`].
    pub string_texts: Vec<Entity>,

    /// All entities that link to another tooltip.
    /// That is [`TooltipTermLinkRecursive`].
    pub term_texts: Vec<Entity>,

    /// All entities that highlight panels
    /// That is [`TooltipHighlightLink`].
    pub highlight_texts: Vec<Entity>,
}

#[derive(SystemParam)]
/// Add this to your query parameters to conveniently get widgets child entities by component.
/// use [`tooltip_child_entities`] method to gather the information.
pub struct TooltipEntitiesParam<'w, 's> {
    ancestor_query: Query<'w, 's, &'static ChildOf>,

    title_node_query: Query<'w, 's, Entity, With<TooltipTitleNode>>,
    title_text_query: Query<'w, 's, Entity, With<TooltipTitleText>>,

    text_node_query: Query<'w, 's, Entity, With<TooltipTextNode>>,

    string_texts_query: Query<'w, 's, Entity, With<TooltipStringText>>,
    links_query: Query<'w, 's, Entity, With<TooltipTermLinkRecursive>>,
    highlights_query: Query<'w, 's, Entity, With<TooltipHighlightLink>>,
}

impl<'w, 's> TooltipEntitiesParam<'w, 's> {
    /// Given a [`Tooltip`] entity it gather all child Entities and
    /// store it under a [`TooltipEntities`] struct.
    ///
    /// Result will be none if the entity doesn't have expected children.
    pub fn tooltip_child_entities(self, entity: Entity) -> Option<TooltipEntites> {
        let mut title_node = None;
        for title in self.title_node_query {
            if entity == self.ancestor_query.root_ancestor(title) {
                title_node = Some(title);
                break;
            }
        }

        let mut title_text = None;
        for title in self.title_text_query {
            if entity == self.ancestor_query.root_ancestor(title) {
                title_text = Some(title);
                break;
            }
        }

        let mut text_node = None;
        for text in self.text_node_query {
            if entity == self.ancestor_query.root_ancestor(text) {
                text_node = Some(text);
            }
        }

        let mut string_texts = Vec::new();
        for text in self.string_texts_query {
            if entity == self.ancestor_query.root_ancestor(text) {
                string_texts.push(text);
            }
        }

        let mut link_texts = Vec::new();
        for link in self.links_query {
            if entity == self.ancestor_query.root_ancestor(link) {
                link_texts.push(link);
            }
        }

        let mut highlight_texts = Vec::new();
        for highlight in self.highlights_query {
            if entity == self.ancestor_query.root_ancestor(highlight) {
                highlight_texts.push(highlight);
            }
        }
        Some(TooltipEntites {
            title_node: r!(title_node),
            title_text: r!(title_text),
            tooltip_text_node: r!(text_node),
            string_texts,
            term_texts: link_texts,
            highlight_texts,
        })
    }
}
