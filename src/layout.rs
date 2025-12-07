//! Components you can query or observer in order to change appearance
//! of the tooltips, can be used in queries when obeserving the [`crate::TooltipSpawned`] event.
//!

use bevy_ecs::component::Component;

/// Marker for the [`crate::Tooltip`] title node
#[derive(Debug, Component)]
pub struct TooltipTitleNode;

/// Marker for the [`crate::Tooltip`] title text this will be place in `TooltipTitleNode`
#[derive(Debug, Component)]
pub struct TooltipTitleText;

/// Marker for the [`crate::Tooltip`] info node, that is the node that holds all non title text
#[derive(Debug, Component)]
pub struct TooltipTextNode;

/// Marker for the [`crate::Tooltip`] texts that is not interactable
#[derive(Debug, Component)]
pub struct TooltipStringText;
