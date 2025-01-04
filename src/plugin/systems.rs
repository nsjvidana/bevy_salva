use crate::fluid::{FluidDensity, FluidNonPressureForces, FluidParticlePositions, SalvaFluidHandle};
use bevy::prelude::{error, warn, Changed, Commands, Entity, Query, RemovedComponents, Res, ResMut, Time, With, Without};
use salva::math::Vector;
use salva::object::interaction_groups::InteractionGroups;
use salva::{math::Point, object::Fluid};

use crate::fluid::{AppendNonPressureForces, RemoveNonPressureForcesAt};
use crate::math::Vect;
use crate::plugin::salva_context::SalvaContext;
use crate::plugin::{DefaultSalvaContext, SalvaContextAccess, SalvaContextEntityLink, WriteDefaultSalvaContext, WriteSalvaContext};

pub fn init_fluids(
    mut commands: Commands,
    mut new_fluids: Query<
        (
            Entity,
            &FluidParticlePositions,
            Option<&FluidDensity>,
            Option<&mut FluidNonPressureForces>,
            Option<&SalvaContextEntityLink>
        ),
        Without<SalvaFluidHandle>,
    >,
    q_default_context: Query<Entity, With<DefaultSalvaContext>>,
    mut q_contexts: Query<&mut SalvaContext>,
) {
    for (
        entity,
        particle_positions,
        density,
        nonpressure_forces,
        context_link
    ) in new_fluids.iter_mut() {
        let mut entity_cmd = commands.entity(entity);

        let density = density.map_or_else(|| 1000.0, |d| d.density0);

        #[cfg(feature = "dim2")]
        let particle_positions: Vec<_> = particle_positions
            .positions
            .iter()
            .map(|v| Point::new(v.x, v.y))
            .collect();
        #[cfg(feature = "dim3")]
        let particle_positions: Vec<_> = particle_positions
            .positions
            .iter()
            .map(|v| Point::new(v.x, v.y, v.z))
            .collect();

        let context_entity = context_link.map_or_else(
            || {
                let context_entity = q_default_context.get_single().ok()?;
                entity_cmd.insert(SalvaContextEntityLink(context_entity));
                Some(context_entity)
            },
            |link| Some(link.0)
        );

        let Some(context_entity) = context_entity else {
            continue;
        };

        let Ok(mut context) = q_contexts.get_mut(context_entity) else {
            error!("Couldn't find salva context entity {context_entity} while initializing {entity}");
            continue;
        };

        let mut salva_fluid = Fluid::new(
            particle_positions,
            context.liquid_world.particle_radius(),
            density,
            InteractionGroups::default(), //TODO: make this an optional ecs component instead
        );
        if let Some(mut nonpressure_forces) = nonpressure_forces {
            salva_fluid
                .nonpressure_forces
                .append(&mut nonpressure_forces.0);
        }
        let fluid_handle = context.liquid_world.add_fluid(salva_fluid);
        entity_cmd.insert(SalvaFluidHandle(fluid_handle));
        context.entity2fluid.insert(entity, fluid_handle);
    }
}

pub fn apply_fluid_user_changes(
    mut context_writer: WriteSalvaContext,
    mut append_q: Query<
        (&SalvaFluidHandle, &SalvaContextEntityLink, &mut AppendNonPressureForces),
        Changed<AppendNonPressureForces>,
    >,
    mut remove_at_q: Query<
        (&SalvaFluidHandle, &SalvaContextEntityLink, &mut RemoveNonPressureForcesAt),
        Changed<RemoveNonPressureForcesAt>,
    >,
) {
    // Handles nonpressure forces the user wants to append to fluids
    for (handle, link, mut appends) in append_q.iter_mut() {
        let mut context = context_writer.context(link);
        context
            .liquid_world
            .fluids_mut()
            .get_mut(handle.0)
            .unwrap()
            .nonpressure_forces
            .append(&mut appends.0);
    }

    // Handles nonpressure forces the user wants to remove from fluids
    for (handle, link, mut removals) in remove_at_q.iter_mut() {
        let mut context = context_writer.context(link);
        let nonpressure_forces = &mut context
            .liquid_world
            .fluids_mut()
            .get_mut(handle.0)
            .unwrap()
            .nonpressure_forces;

        for i in removals.0.iter() { nonpressure_forces.remove(*i); }
        removals.0.clear();
    }
}

pub fn sync_removals(
    mut removed_particle_positions: RemovedComponents<FluidParticlePositions>,
    mut removed_fluids: RemovedComponents<SalvaFluidHandle>,
    mut context_writer: WriteSalvaContext
) {
    //remove fluids whos entities had their salva fluid handle or fluid particle components removed
    for entity in removed_fluids
        .read()
        .chain(removed_particle_positions.read())
    {
        if let Some((mut context, handle)) = context_writer.salva_context
            .iter_mut()
            .find_map(|mut context| {
                context.entity2fluid.remove(&entity).map(|h| (context, h))
            })
        {
            context.liquid_world.remove_fluid(handle);
        }
    }
}

// WIP: for now, just assume that everything is run in bevy's fixed update step
pub fn step_simulation(
    mut salva_context: Query<&mut SalvaContext>,
    time: Res<Time>,
) {
    for mut context in salva_context.iter_mut() {
        #[cfg(feature = "dim2")]
        context.step(
            time.delta_secs(),
            &Vector::new(0., -9.81) //TODO: make gravity customizable
        );
        #[cfg(feature = "dim3")]
        context.step(
            time.delta_secs(),
            &Vector::new(0., -9.81, 0.)
        );
    }
}

pub fn writeback_particle_positions(
    read_context: SalvaContextAccess,
    mut fluid_pos_q: Query<(&SalvaFluidHandle, &SalvaContextEntityLink, &mut FluidParticlePositions)>,
) {
    for (handle, link, mut particle_positions) in fluid_pos_q.iter_mut() {
        let context = read_context.context(link);
        let positions = &context.liquid_world.fluids().get(handle.0).unwrap().positions;
        
        #[cfg(feature = "dim2")]
        {
            particle_positions.positions = positions.iter().map(|v| Vect::new(v.x, v.y)).collect();
        }
        #[cfg(feature = "dim3")]
        {
            particle_positions.positions = positions.iter().map(|v| Vect::new(v.x, v.y, v.z)).collect();
        }
    }
}

