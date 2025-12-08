//! Terms is how tooltips find out what to display given a word to link.

use bevy_ecs::{
    component::Component,
    entity::Entity,
    observer::On,
    system::{Commands, Res},
};
use bevy_time::{Timer, TimerMode};
use tiny_bail::prelude::*;

use crate::{
    ActivationMethod, TooltipConfiguration, TooltipLinkTimer, text_observer::TextHoveredOver,
};

/// Place this on a node or text that you want to spawn a Tooltip.
/// The tooltip displayed will be the contents of [`TooltipMap`].
#[derive(Debug, Component)]
pub struct TooltipTermLink {
    pub(crate) linked_string: String,
}

impl TooltipTermLink {
    /// Create a new link component
    pub fn new(linked_string: impl ToString) -> Self {
        Self {
            linked_string: linked_string.to_string(),
        }
    }
    /// The string that is used to look up the term
    pub fn linked_string(&self) -> &str {
        &self.linked_string
    }
}

/// This is used for putting links of tooltips in tooltips
/// Should not be created by end users but can safely read if you are interested in recursive case
/// Recursive case may be treated seperately in future such as shorter hover times.
#[derive(Debug, Component)]
pub struct TooltipTermLinkRecursive {
    pub(crate) parent_entity: Entity,
    pub(crate) linked_string: String,
}

impl TooltipTermLinkRecursive {
    /// Creates a new link with the given string and parent entity.
    pub(crate) fn new(parent_entity: Entity, linked_string: String) -> Self {
        Self {
            parent_entity,
            linked_string,
        }
    }
    /// The string that is used to look up the term.
    pub fn linked_string(&self) -> &str {
        &self.linked_string
    }

    /// The [`ToolTip`] that holds this link.
    pub fn parent_entity(&self) -> Entity {
        self.parent_entity
    }
}

/// This triggers for [`Tooltip`] links
/// If configured to display on hover this will add a [`TooltipLinkTimer`] that unless pointer moves
/// away from will spawn a [`Tooltip`].
pub(crate) fn hover_time_spawn(
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
