use bevy::prelude::{Commands, Component, Entity, Query, Res, ResMut, Time, With, Without};
use bevy_rapier::geometry::RapierColliderHandle;
use bevy_rapier::parry::math::Point;
use bevy_rapier::plugin::{DefaultRapierContext, RapierConfiguration, ReadDefaultRapierContext, WriteRapierContext};
use bevy_rapier::prelude::{CollisionGroups, RapierContextAccess, RapierContextEntityLink};
use bitflags::Flags;
use salva::integrations::rapier::{ColliderCouplingSet, ColliderSampling};
use salva::math::Vector;
use salva::object::{Boundary, BoundaryHandle};
use salva::object::interaction_groups::InteractionGroups;
use crate::plugin::{DefaultSalvaContext, SalvaConfiguration, SalvaContext, SalvaContextEntityLink, SalvaContextInitialization, SimulationToRenderTime, WriteSalvaContext};
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
/// entity a [`SalvaContext`] entity has its simulation coupled with.
///
/// Also contains the coupling manager ([`ColliderCouplingSet`]) needed for coupling rapier with salva.
#[derive(Component)]
pub struct SalvaRapierCoupling {
    /// The [`RapierContext`] entity that this [`SalvaContext`] entity is coupled with
    pub rapier_context_entity: Entity,
    /// The structure used in coupling rapier colliders with salva fluid boundaries to simulate.
    /// rigidbody-fluid interactions
    pub coupling: ColliderCouplingSet,
}

// WIP: for now, just assume that everything is run in bevy's fixed update step
pub fn step_simulation_rapier_coupling(
    mut salva_context_q: Query<(
        &mut SalvaContext,
        &mut SalvaRapierCoupling,
        &SalvaConfiguration,
        &mut SimulationToRenderTime
    )>,
    mut write_rapier_context: WriteRapierContext,
    time: Res<Time>,
) {
    for (
        mut context,
        mut link,
        config,
        mut sim_to_render_time
    ) in salva_context_q.iter_mut() {
        let rapier_context = write_rapier_context
            .try_context_from_entity(link.rapier_context_entity)
            .expect("Couldn't find RapierContext coupled to SalvaContext entity {entity}")
            .into_inner();

        context.step_with_coupling(
            &time,
            &config.gravity.into(),
            config.timestep_mode.clone(),
            &mut sim_to_render_time,
            &mut link.coupling
                .as_manager_mut(&rapier_context.colliders, &mut rapier_context.bodies),
        );
    }
}

/// The system responsible for sampling/coupling rapier colliders for rapier-salva coupling
/// by converting them into fluid boundaries.
pub fn sample_rapier_colliders(
    mut commands: Commands,
    colliders: Query<
        (
            Entity,
            &RapierContextEntityLink,
            Option<&SalvaContextEntityLink>,
            &RapierColliderHandle,
            &RapierColliderSampling,
            Option<&CollisionGroups>,
        ),
        Without<ColliderBoundaryHandle>,
    >,
    mut rapier_coupling_q: Query<&mut SalvaRapierCoupling>,
    q_default_context: Query<Entity, With<DefaultSalvaContext>>,
    mut context_writer: WriteSalvaContext,
    rapier_context_access: RapierContextAccess,
) {
    for (
        entity,
        rapier_link,
        salva_link,
        co_handle,
        sampling,
        collision_groups,
    ) in colliders.iter() {
        let mut entity_cmd = commands.entity(entity);
        let salva_link = salva_link.map_or_else(
            || {
                let context_entity = q_default_context.get_single().unwrap();
                entity_cmd.insert(SalvaContextEntityLink(context_entity));
                SalvaContextEntityLink(context_entity)
            },
            |link| *link
        );

        let mut salva_context = context_writer.context(&salva_link);
        let radius = salva_context.liquid_world.particle_radius();
        let coupling = &mut rapier_coupling_q.get_mut(salva_link.0).unwrap().coupling;

        let rapier_context = rapier_context_access.context(rapier_link);
        let co = rapier_context.colliders.get(co_handle.0).unwrap();

        let bo_handle = salva_context
            .liquid_world
            .add_boundary(Boundary::new(
                Vec::new(),
                collision_groups.map_or_else(
                    || InteractionGroups::default(),
                    |groups| InteractionGroups {
                        memberships: salva::object::interaction_groups::Group::from_bits(groups.memberships.bits())
                            .unwrap(),
                        filter: salva::object::interaction_groups::Group::from_bits(groups.filters.bits())
                            .unwrap(),
                    }
                )
            ));
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

        entity_cmd
            .insert(ColliderBoundaryHandle(bo_handle));
    }
}

/// System that links the default salva context to the default rapier context
#[cfg(feature = "rapier")]
pub fn link_default_contexts(
    mut commands: Commands,
    initialization_data: Res<SalvaContextInitialization>,
    mut default_salva_context: Query<
        (Entity, &mut SalvaConfiguration),
        (With<DefaultSalvaContext>, Without<SalvaRapierCoupling>)
    >,
    mut default_rapier_context: Query<(Entity, &mut RapierConfiguration), With<DefaultRapierContext>>,
) {
    match initialization_data.as_ref() {
        SalvaContextInitialization::NoAutomaticSalvaContext => {}
        SalvaContextInitialization::InitializeDefaultSalvaContext {
            particle_radius: _particle_radius, smoothing_factor: _smoothing_factor
        } => {
            let (salva_context_entity, mut salva_config) = default_salva_context
                .get_single_mut().unwrap();
            let (rapier_context_entity, mut rapier_config) = default_rapier_context
                .get_single_mut().unwrap();
            commands.entity(salva_context_entity)
                .insert(SalvaRapierCoupling {
                    rapier_context_entity,
                    coupling: ColliderCouplingSet::new(),
                });
            salva_config.gravity = rapier_config.gravity;
        }
    }
}
