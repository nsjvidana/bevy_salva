//TODO: salva context, default read/write, SalvaEntity query data

use salva::math::Vector;
use bevy::prelude::{Component, Entity, Mut, Query, Reflect, Resource, With};
use salva::LiquidWorld;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use bevy::ecs::query::QueryData;
use bevy::ecs::system::SystemParam;
use salva::object::FluidHandle;

#[cfg(feature = "rapier")]
use salva::integrations::rapier::ColliderCouplingSet;
#[cfg(feature = "rapier")]
use bevy_rapier::plugin::RapierContext;
use salva::coupling::CouplingManager;

#[derive(Component)]
pub struct SalvaContext {
    pub liquid_world: LiquidWorld,
    pub entity2fluid: HashMap<Entity, FluidHandle>,
}

impl SalvaContext {
    pub fn step_with_coupling(
        &mut self, dt: f32,
        gravity: &Vector<f32>,
        coupling: &mut impl CouplingManager
    ) {
        self.liquid_world.step_with_coupling(
            dt,
            gravity,
            coupling
        );
    }

    pub fn step(&mut self, dt: f32, gravity: &Vector<f32>) {
        self.liquid_world.step(
            dt,
            gravity
        );
    }
}

/// This is a component applied to any entity containing a salva handle component.
/// The inner Entity referred to has the component [`SalvaContext`] responsible for handling
/// its salva data.
#[derive(Component, Reflect, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SalvaContextEntityLink(pub Entity);

/// ECS query data that queries for entities that contain a salva handle component.
/// Contains the link the entity has to a salva context.
#[derive(QueryData)]
pub struct SalvaEntity {
    pub entity: Entity,
    pub salva_context_link: &'static SalvaContextEntityLink
}

/// Marker component for to access the default [`SalvaContext`].
///
/// This is used by [`systemparams::ReadDefaultSalvaContext`] and other default accesses
/// to help with getting a reference to the correct SalvaContext.
///
/// If you're making a library, you might be interested in [`SalvaContextEntityLink`]
/// and leverage a [`Query<&SalvaContext>`] to find the correct [`SalvaContext`] of an entity.
#[derive(Component, Reflect, Debug, Clone, Copy)]
pub struct DefaultSalvaContext;

/// Utility [`SystemParam`] to easily access the single default [`SalvaContext`] immutably.
///
/// SAFETY: Dereferencing this struct will panic if its underlying query fails.
/// See [`SalvaContextAccess`] for a safer alternative.
#[derive(SystemParam)]
pub struct ReadDefaultSalvaContext<'w, 's, T: Component = DefaultSalvaContext> {
    salva_context: Query<'w, 's, &'static SalvaContext, With<T>>,
}

impl<'w, 's, T: Component> ReadDefaultSalvaContext<'w, 's, T> {
    /// Use this method if you only have one [`SalvaContext`].
    ///
    /// SAFETY: This method will panic if its underlying query fails.
    /// See [`SalvaContextAccess`] for a safe alternative.
    pub fn single(&'_ self) -> &SalvaContext {
        self.salva_context.single()
    }
}

impl<'w, 's> Deref for ReadDefaultSalvaContext<'w, 's> {
    type Target = SalvaContext;

    /// Use this method if you only have one [`SalvaContext`].
    ///
    /// SAFETY: This method will panic if its underlying query fails.
    /// See [`SalvaContextAccess`] for a safe alternative.
    fn deref(&self) -> &Self::Target {
        self.salva_context.single()
    }
}

/// Utility [`SystemParam`] to easily access the single default [`SalvaContext`] mutably.
///
/// SAFETY: Dereferencing this struct will panic if its underlying query fails.
/// See [`SalvaContextAccess`] for a safer alternative.
#[derive(SystemParam)]
pub struct WriteDefaultSalvaContext<'w, 's, T: Component = DefaultSalvaContext> {
    salva_context: Query<'w, 's, &'static mut SalvaContext, With<T>>,
}

impl<'w, 's, T: Component> Deref for WriteDefaultSalvaContext<'w, 's, T> {
    type Target = SalvaContext;

    /// Use this method if you only have one [`SalvaContext`].
    ///
    /// SAFETY: This method will panic if its underlying query fails.
    /// See [`SalvaContextAccess`] for a safe alternative.
    fn deref(&self) -> &Self::Target {
        self.salva_context.single()
    }
}

impl<'w, 's> DerefMut for WriteDefaultSalvaContext<'w, 's> {
    /// Use this method if you only have one [`SalvaContext`].
    ///
    /// SAFETY: This method will panic if its underlying query fails.
    /// See [`WriteSalvaContext`] for a safe alternative.
    fn deref_mut(&mut self) -> &mut Self::Target {
        // TODO: should we cache the result ?
        self.salva_context.single_mut().into_inner()
    }
}

/// Utility [`SystemParam`] to easily access any [`SalvaContext`] immutably
#[derive(SystemParam)]
pub struct SalvaContextAccess<'w, 's> {
    /// Query used to retrieve a [`SalvaContext`].
    /// It's helpful to iterate over every salva contexts,
    /// or get a handle over a specific context, for example through:
    /// - a marker component such as [`DefaultSalvaContext`]
    /// - a [`SalvaContextEntityLink`]. See [context](SalvaContextAccess::context)
    pub salva_context: Query<'w, 's, &'static SalvaContext>,
}

impl<'w, 's> SalvaContextAccess<'w, 's> {
    /// Retrieves the salva context responsible for the entity owning the given [`SalvaContextEntityLink`].
    ///
    /// SAFETY: This method will panic if its underlying query fails.
    /// See [`Self::try_context`] for a safe alternative.
    pub fn context(&self, link: &SalvaContextEntityLink) -> &'_ SalvaContext {
        self.try_context(link)
            .expect("SalvaContextEntityLink.0 refers to an entity without SalvaContext.")
    }

    /// Retrieves the salva context responsible for the entity owning the given [`SalvaContextEntityLink`].
    pub fn try_context(&self, link: &SalvaContextEntityLink) -> Option<&'_ SalvaContext> {
        self.salva_context.get(link.0).ok()
    }
}

impl<'w, 's> Deref for SalvaContextAccess<'w, 's> {
    type Target = SalvaContext;

    fn deref(&self) -> &Self::Target {
        self.salva_context.single()
    }
}

/// Utility [`SystemParam`] to easily access any [`SalvaContext`] mutably
/// 
/// PERFORMANCE: this [`SystemParam`] queries ALL salva contexts mutably, which
/// can limit the ability to parallelize systems in some cases. 
#[derive(SystemParam)]
pub struct WriteSalvaContext<'w, 's> {
    /// Query used to retrieve a [`SalvaContext`].
    /// It's helpful to iterate over every salva contexts,
    /// or get a handle over a specific context, for example through:
    /// - a marker component such as [`DefaultSalvaContext`]
    /// - a [`SalvaContextEntityLink`]. See [context](SalvaContextAccess::context)
    pub salva_context: Query<'w, 's, &'static mut SalvaContext>,
}

impl<'w, 's> WriteSalvaContext<'w, 's> {
    /// Retrieves the salva context responsible for the entity owning the given [`SalvaContextEntityLink`].
    ///
    /// SAFETY: This method will panic if its underlying query fails.
    /// See [`Self::try_context`] for a safe alternative.
    pub fn context(&mut self, link: &SalvaContextEntityLink) -> Mut<SalvaContext> {
        self.try_context(link)
            .expect("SalvaContextEntityLink.0 refers to an entity without SalvaContext.")
    }

    /// Retrieves the salva context responsible for the entity owning the given [`SalvaContextEntityLink`].
    pub fn try_context(&mut self, link: &SalvaContextEntityLink) -> Option<Mut<SalvaContext>> {
        self.salva_context.get_mut(link.0).ok()
    }

    /// Retrieves the salva context component on this [`Entity`].
    ///
    /// Calling this method on a salva managed entity (rigid body, collider, joints...) will fail.
    /// Given entity should have a [`SalvaContext`].
    pub fn try_context_from_entity(
        &mut self,
        salva_context_entity: Entity,
    ) -> Option<Mut<SalvaContext>> {
        self.salva_context.get_mut(salva_context_entity).ok()
    }
}

