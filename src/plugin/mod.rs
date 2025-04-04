pub use crate::fluid::AppendNonPressureForces;
pub use crate::fluid::RemoveNonPressureForcesAt;
pub use self::plugin::{
    SalvaPhysicsPlugin, SalvaSimulationSet, SalvaContextInitialization
};
pub use salva_context::*;
pub use configuration::*;

#[allow(clippy::type_complexity)]
pub mod systems;
#[allow(clippy::module_inception)]
mod plugin;
mod salva_context;
mod configuration;