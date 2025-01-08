use bevy::prelude::Component;
use crate::math::Vect;

/// This structure is used when [`TimestepMode::Interpolated`] is
/// enabled for a [`SalvaContext`] entity.
#[derive(Component, Default)]
pub struct SimulationToRenderTime {
    pub diff: f32
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum TimestepMode {
    /// Use a fixed timestep: the physics simulation will be advanced by the fixed value
    Fixed {
        /// The physics simulation will be advanced by this amount of time at each Bevy tick.
        dt: f32,
        /// This number of substeps of length `dt / substeps` will be performed at each Bevy tick.
        substeps: usize,
    },
    Variable {
        /// Maximum amount of time the physics simulation may be advanced at each Bevy tick.
        max_dt: f32,
        /// Multiplier controlling if the physics simulation should advance faster (> 1.0),
        /// at the same speed (= 1.0) or slower (< 1.0) than the real time.
        time_scale: f32,
        /// The number of substeps that will be performed at each tick.
        substeps: usize,
    },
    /// Use a fixed timestep equal to `IntegrationParameters::dt`, but don't step if the
    /// physics simulation advanced by a time greater than the real-world elapsed time multiplied by `time_scale`.
    Interpolated {
        /// The physics simulation will be advanced by this total amount at each Bevy tick, unless
        /// the physics simulation time is ahead of a the real time.
        dt: f32,
        /// Multiplier controlling if the physics simulation should advance faster (> 1.0),
        /// at the same speed (= 1.0) or slower (< 1.0) than the real time.
        time_scale: f32,
        /// The number of substeps that will be performed whenever the physics simulation is advanced.
        substeps: usize,
    },
}

impl Default for TimestepMode {
    fn default() -> Self {
        TimestepMode::Variable {
            max_dt: 1.0 / 60.0,
            time_scale: 1.0,
            substeps: 1,
        }
    }
}

/// A component required for all entities that have a [`SalvaContext`].
#[derive(Component, Copy, Clone, Debug)]
pub struct SalvaConfiguration {
    /// Specifies the gravity of the physics simulation.
    pub gravity: Vect,
    /// If this is `false`, the simulation won't step, and another system would have to be set up
    /// for stepping the [`LiquidWorld`] that this [`SalvaContext`] entity has.
    ///
    /// This is typically set to `false` whenever a [`SalvaContext`] needs to be
    /// coupled to another physics engine of some kind.
    pub default_step_active: bool,
    pub timestep_mode: TimestepMode,
}

impl SalvaConfiguration {
    pub fn new(gravity: Vect) -> Self {
        Self {
            gravity,
            ..Default::default()
        }
    }
}

impl Default for SalvaConfiguration {
    fn default() -> Self {
        Self {
            gravity: Vect::Y * -9.81,
            default_step_active: true,
            timestep_mode: TimestepMode::default()
        }
    }
}
