//! Components you can query or observer in order to change appearance
//! of the tooltips

use bevy_ecs::component::Component;

/// Marker for the `Tooltip` title node
#[derive(Debug, Component)]
pub struct TooltipTitleNode;

/// Marker for the `Tooltip` title text this will be place in `TooltipTitleNode`
#[derive(Debug, Component)]
pub struct TooltipTitleText;

/// Marker for the `Tooltip` info node, that is the node that holds all non title text
#[derive(Debug, Component)]
pub struct TooltipTextNode;

/// Marker for the `Tooltip` texts that is not interactable
#[derive(Debug, Component)]
pub struct TooltipStringText;
