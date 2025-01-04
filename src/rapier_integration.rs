use bevy::prelude::{Commands, Component, Entity, Query, Res, ResMut, Time, With, Without};
use bevy_rapier::geometry::RapierColliderHandle;
use bevy_rapier::parry::math::Point;
use bevy_rapier::plugin::{DefaultRapierContext, ReadDefaultRapierContext, WriteRapierContext};
use salva::integrations::rapier::{ColliderCouplingSet, ColliderSampling};
use salva::math::Vector;
use salva::object::{Boundary, BoundaryHandle};
use salva::object::interaction_groups::InteractionGroups;
use crate::plugin::{DefaultSalvaContext, SalvaContext, SalvaContextEntityLink, SalvaContextInitialization, WriteSalvaContext};
#[allow(unused_imports)]
use crate::plugin::SalvaPhysicsPlugin;

pub enum ColliderSamplingMethod {
    /// Collider shape is approximated for the fluid simulation in a way that keeps its shape consistent.
    /// The shape is determined using [`salva3d::sampling::shape_surface_ray_sample`]
    ///
    /// Good for smaller objects with finer details. Larger objects cause performance issues.
    Static,
    /// Collider shape is approximated on-the-fly as fluid particles make contact with it.
    ///
    /// Performance is more consistent for shapes of any size at the cost of less detail.
    DynamicContact,
    /// Custom collider shape approximated with the given sample points in local-space.
    ///
    /// It is recommended that the points are separated by a distance smaller or equal to twice
    /// the particle radius used to initialize the fluid simulation world.
    /// The default particle radius is [`SalvaPhysicsPlugin::DEFAULT_PARTICLE_RADIUS`].
    CustomStatic(Vec<Point<f32>>),
}

impl Default for ColliderSamplingMethod {
    fn default() -> Self {
        Self::DynamicContact
    }
}

#[derive(Component, Default)]
pub struct RapierColliderSampling {
    pub sampling_method: ColliderSamplingMethod,
}

#[derive(Component)]
pub struct ColliderBoundaryHandle(pub BoundaryHandle);

/// The component added to [`SalvaContext`] entities that declares which [`RapierContext`]
/// entity the [`SalvaContext`] entity has its simulation coupled with.
///
/// Also contains the coupling manager needed for coupling rapier with salva.
#[derive(Component)]
pub struct SalvaRapierCouplingLink {
    pub rapier_context_entity: Entity,
    pub coupling: ColliderCouplingSet,
}

// WIP: for now, just assume that everything is run in bevy's fixed update step
pub fn step_simulation_rapier_coupling(
    mut salva_context_q: Query<(&mut SalvaContext, &mut SalvaRapierCouplingLink)>,
    mut write_rapier_context: WriteRapierContext,
    time: Res<Time>,
) {
    for (mut context, mut link) in salva_context_q.iter_mut() {
        let rapier_context = write_rapier_context
            .try_context_from_entity(link.rapier_context_entity)
            .expect("Couldn't find RapierContext coupled to SalvaContext entity {entity}")
            .into_inner();

        #[cfg(feature = "dim2")]
        context.step_with_coupling(
            time.delta_secs(),
            &Vector::new(0., -9.81), //TODO: make gravity customizable
            &mut link.coupling
                .as_manager_mut(&rapier_context.colliders, &mut rapier_context.bodies),
        );
        #[cfg(feature = "dim3")]
        context.step_with_coupling(
            time.delta_secs(),
            &Vector::new(0., -9.81, 0.), //TODO: make gravity customizable
            &mut link.coupling
                .as_manager_mut(&rapier_context.colliders, &mut rapier_context.bodies),
        );
    }
}

pub fn sample_rapier_colliders(
    mut commands: Commands,
    colliders: Query<
        (Entity, &RapierColliderHandle, &RapierColliderSampling, &SalvaContextEntityLink),
        Without<ColliderBoundaryHandle>,
    >,
    mut rapier_coupling_q: Query<&mut SalvaRapierCouplingLink>,
    mut context_writer: WriteSalvaContext,
    rapier_context: ReadDefaultRapierContext,
) {
    for (entity, co_handle, sampling, salva_link) in colliders.iter() {
        let mut salva_context = context_writer.context(salva_link);
        let coupling = &mut rapier_coupling_q.get_mut(salva_link.0).unwrap().coupling;

        let radius = salva_context.liquid_world.particle_radius();
        let co = rapier_context.colliders.get(co_handle.0).unwrap();
        let bo_handle = salva_context
            .liquid_world
            .add_boundary(Boundary::new(Vec::new(), InteractionGroups::default()));
        coupling.register_coupling(
            bo_handle,
            co_handle.0,
            match &sampling.sampling_method {
                ColliderSamplingMethod::Static => {
                    let samples =
                        salva::sampling::shape_surface_ray_sample(co.shape(), radius).unwrap();
                    ColliderSampling::StaticSampling(samples)
                }
                ColliderSamplingMethod::DynamicContact => {
                    ColliderSampling::DynamicContactSampling
                }
                ColliderSamplingMethod::CustomStatic(samples) => {
                    ColliderSampling::StaticSampling(samples.clone())
                }
            },
        );

        commands
            .get_entity(entity)
            .unwrap()
            .insert(ColliderBoundaryHandle(bo_handle));
    }
}

/// System that links the default salva context to the default rapier context
#[cfg(feature = "rapier")]
pub fn link_default_contexts(
    mut commands: Commands,
    initialization_data: Res<SalvaContextInitialization>,
    default_salva_context: Query<Entity, (With<DefaultSalvaContext>, Without<SalvaRapierCouplingLink>)>,
    default_rapier_context: Query<Entity, With<DefaultRapierContext>>,
) {
    match initialization_data.as_ref() {
        SalvaContextInitialization::NoAutomaticSalvaContext => {}
        SalvaContextInitialization::InitializeDefaultSalvaContext {
            particle_radius: _particle_radius, smoothing_factor: _smoothing_factor
        } => {
            commands.entity(default_salva_context.get_single().unwrap())
                .insert(SalvaRapierCouplingLink {
                    rapier_context_entity: default_rapier_context.get_single().unwrap(),
                    coupling: ColliderCouplingSet::new(),
                });
        }
    }
}
