use bevy::prelude::{Component, Reflect};
use crate::math::Vect;

//TODO: implement TimestepMode here

/// A component required for all entities that have a [`SalvaContext`].
#[derive(Component, Copy, Clone, Debug, Reflect)]
pub struct SalvaConfiguration {
    /// Specifies the gravity of the physics simulation.
    pub gravity: Vect,
    /// If this is `false`, the simulation won't step, and another system would have to be set up
    /// for stepping the [`LiquidWorld`] that this [`SalvaContext`] entity has.
    ///
    /// This is typically set to `false` whenever a [`SalvaContext`] needs to be
    /// coupled to another physics engine of some kind.
    pub default_step_active: bool
}

impl Default for SalvaConfiguration {
    fn default() -> Self {
        Self {
            gravity: Vect::Y * -9.81,
            default_step_active: true
        }
    }
}
