use std::collections::HashMap;

use crate::systems;
use bevy::prelude::{IntoSystemSetConfigs, SystemSet, TransformSystem};
use bevy::{
    app::{Plugin, PostUpdate},
    ecs::{
        intern::Interned,
        schedule::{ScheduleLabel, SystemConfigs},
    },
    prelude::{Component, Entity, IntoSystemConfigs, Resource},
};
use bevy_rapier3d::parry::math::Vector;
use bevy_rapier3d::plugin::PhysicsSet;
use bevy_rapier3d::prelude::RapierContext;
use bevy_rapier3d::rapier::dynamics::RigidBodySet;
use bevy_rapier3d::rapier::geometry::ColliderSet;
use salva3d::integrations::rapier::ColliderCouplingSet;
use salva3d::{
    math::Real,
    object::FluidHandle,
    solver::{NonPressureForce, PressureSolver},
    LiquidWorld,
};

//TODO: use a feature for enabling coupling with bevy_rapier
pub struct SalvaPhysicsPlugin<S: PressureSolver + Send + Sync + 'static> {
    schedule: Interned<dyn ScheduleLabel>,
    default_rapier_coupling_config: bool,
    solver: S,
    particle_radius: Real,
    smoothing_factor: Real,
}

impl<S: PressureSolver + Send + Sync + 'static> SalvaPhysicsPlugin<S> {
    pub const DEFAULT_PARTICLE_RADIUS: Real = 0.05;
    pub const DEFAULT_SMOOTHING_FACTOR: Real = 2.0;

    pub fn new(solver: S) -> Self {
        Self {
            schedule: PostUpdate.intern(),
            default_rapier_coupling_config: true,
            solver,
            particle_radius: Self::DEFAULT_PARTICLE_RADIUS,
            smoothing_factor: Self::DEFAULT_SMOOTHING_FACTOR,
        }
    }

    pub fn in_schedule(mut self, schedule: impl ScheduleLabel) -> Self {
        self.schedule = schedule.intern();
        self
    }

    pub fn with_solver(mut self, solver: S) -> Self {
        self.solver = solver;
        self
    }

    pub fn with_particle_radius(mut self, particle_radius: Real) -> Self {
        self.particle_radius = particle_radius;
        self
    }

    pub fn with_smoothing_factor(mut self, smoothing_factor: Real) -> Self {
        self.smoothing_factor = smoothing_factor;
        self
    }

    pub fn use_default_rapier_coupling(mut self, use_default_coupling: bool) -> Self {
        self.default_rapier_coupling_config = use_default_coupling;
        self
    }

    pub fn get_systems(set: SalvaSimulationSet) -> SystemConfigs {
        match set {
            SalvaSimulationSet::SyncBackend => (
                systems::sync_removals,
                systems::init_fluids,
                systems::apply_nonpressure_force_changes,
                systems::sample_rapier_colliders,
            )
                .chain()
                .in_set(SalvaSimulationSet::SyncBackend),
            SalvaSimulationSet::StepSimulation => {
                (systems::step_simulation).in_set(SalvaSimulationSet::StepSimulation)
            }
            SalvaSimulationSet::Writeback => (systems::writeback_particle_positions,)
                .chain()
                .in_set(SalvaSimulationSet::Writeback),
            _ => todo!(),
        }
    }
}

#[derive(Resource)]
pub struct SalvaContext {
    pub liquid_world: LiquidWorld,
    pub coupling: ColliderCouplingSet,
    pub entity2fluid: HashMap<Entity, FluidHandle>,
}

impl SalvaContext {
    pub fn step(&mut self, dt: f32, gravity: &Vector<f32>, rapier_context: &mut RapierContext) {
        self.liquid_world.step_with_coupling(
            dt,
            gravity,
            &mut self
                .coupling
                .as_manager_mut(&rapier_context.colliders, &mut rapier_context.bodies),
        );
    }
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum SalvaSimulationSet {
    SyncBackend,
    StepSimulation,
    Writeback,
}

impl<S: PressureSolver + Send + Sync + 'static> Plugin for SalvaPhysicsPlugin<S> {
    fn build(&self, app: &mut bevy::prelude::App) {
        // SAFETY: this is fine because self.solver is private, meaning that
        //         self.solver cannot be accessed after the app closes
        let solver: S = unsafe { std::mem::transmute_copy(&self.solver) };

        app.insert_resource(SalvaContext {
            liquid_world: LiquidWorld::new(solver, self.particle_radius, self.smoothing_factor),
            entity2fluid: HashMap::default(),
            coupling: ColliderCouplingSet::new(),
        });

        if self.schedule != PostUpdate.intern() {
            app.add_systems(
                PostUpdate,
                (systems::sync_removals,).before(TransformSystem::TransformPropagate),
            );
        }

        if self.default_rapier_coupling_config {
            app.configure_sets(
                self.schedule,
                (
                    SalvaSimulationSet::SyncBackend,
                    SalvaSimulationSet::StepSimulation,
                    SalvaSimulationSet::Writeback,
                )
                    .chain()
                    .before(TransformSystem::TransformPropagate)
                    .after(PhysicsSet::Writeback),
            );

            app.add_systems(
                self.schedule,
                (
                    Self::get_systems(SalvaSimulationSet::SyncBackend),
                    Self::get_systems(SalvaSimulationSet::StepSimulation),
                    Self::get_systems(SalvaSimulationSet::Writeback),
                ),
            );

            //TODO: implement a TimestepMode like how bevy_rapier has it
        }
    }
}

#[derive(Component)]
pub struct AppendNonPressureForces(pub Vec<Box<dyn NonPressureForce>>);

#[derive(Component)]
pub struct RemoveNonPressureForcesAt(pub Vec<usize>);