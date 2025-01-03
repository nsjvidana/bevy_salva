use bevy::prelude::{Commands, Component, Entity, Query, ResMut, Without};
use bevy_rapier::geometry::RapierColliderHandle;
use bevy_rapier::parry::math::Point;
use bevy_rapier::plugin::ReadDefaultRapierContext;
use salva::integrations::rapier::ColliderSampling;
use salva::object::{Boundary, BoundaryHandle};
use salva::object::interaction_groups::InteractionGroups;
use crate::plugin::{SalvaContext, SalvaContextEntityLink, WriteSalvaContext};
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
#[derive(Component)]
pub struct SalvaRapierCouplingLink {
    pub rapier_context_entity: Entity,
}

pub fn sample_rapier_colliders(
    mut commands: Commands,
    colliders: Query<
        (Entity, &RapierColliderHandle, &RapierColliderSampling, &SalvaContextEntityLink),
        Without<ColliderBoundaryHandle>,
    >,
    mut context_writer: WriteSalvaContext,
    rapier_context: ReadDefaultRapierContext,
) {
    for (entity, co_handle, sampling, salva_link) in colliders.iter() {
        let mut salva_context = context_writer.context(salva_link);
        let radius = salva_context.liquid_world.particle_radius();
        let co = rapier_context.colliders.get(co_handle.0).unwrap();
        let bo_handle = salva_context
            .liquid_world
            .add_boundary(Boundary::new(Vec::new(), InteractionGroups::default()));
        salva_context.coupling.register_coupling(
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
