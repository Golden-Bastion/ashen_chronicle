use crate::content::CampaignContent;
use crate::model::{Location, Region, World, WorldMode};
use crate::rng::DeterministicRng;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldGenerationConfig {
    pub region_count: usize,
    pub location_count: usize,
    pub extra_edges: usize,
}

impl Default for WorldGenerationConfig {
    fn default() -> Self {
        Self {
            region_count: 3,
            location_count: 12,
            extra_edges: 8,
        }
    }
}

impl WorldGenerationConfig {
    fn normalized(self) -> Self {
        let location_count = self.location_count.max(1);
        Self {
            region_count: self.region_count.max(1).min(location_count),
            location_count,
            extra_edges: self.extra_edges,
        }
    }
}

/// Generate a deterministic, connected world graph from `seed`.
///
/// The generator only establishes structural world data. Authored campaign
/// content can be anchored into the generated structure with
/// [`place_authored_content`] without changing the generation rules.
pub fn generate_world(
    world_name: impl Into<String>,
    seed: u64,
    config: WorldGenerationConfig,
) -> World {
    let config = config.normalized();
    let mut rng = DeterministicRng::new(seed);
    let mut world = World::new(&world_name.into(), WorldMode::New);

    let mut region_ids = Vec::with_capacity(config.region_count);
    for index in 0..config.region_count {
        let id = world.allocate_id();
        world.regions.push(Region {
            id,
            name: format!("Region {:02}", index + 1),
            description: format!("A generated region shaped by seed {seed:016x}."),
            location_ids: Vec::new(),
        });
        region_ids.push(id);
    }

    for index in 0..config.location_count {
        let region_index = (index * config.region_count) / config.location_count;
        let region_id = region_ids[region_index];
        let id = world.allocate_id();
        world.locations.push(Location {
            id,
            name: format!("Generated Site {:02}", index + 1),
            description: "An unexplored place shaped by the world seed.".to_string(),
            region_id,
            dangerous: rng.next_bool(),
            corpse_ids: Vec::new(),
            exits: Vec::new(),
            spatial_layout: None,
        });
    }

    for index in 1..world.locations.len() {
        add_bidirectional_exit(&mut world, index - 1, index);
    }

    let max_extra_edges = possible_extra_edges(world.locations.len());
    let extra_edges = config.extra_edges.min(max_extra_edges);
    let mut added_edges = 0;
    let mut attempts = 0;
    let max_attempts = extra_edges.saturating_mul(8).saturating_add(16);

    while added_edges < extra_edges && attempts < max_attempts {
        attempts += 1;
        let a = rng.gen_range(0..world.locations.len());
        let b = rng.gen_range(0..world.locations.len());
        if a == b || has_exit(&world.locations[a], world.locations[b].id) {
            continue;
        }
        add_bidirectional_exit(&mut world, a, b);
        added_edges += 1;
    }

    rebuild_region_location_ids(&mut world);
    world
}

/// Place authored campaign locations into deterministic generated slots.
///
/// The first generated region is the anchor for the authored campaign region.
/// Authored locations are ordered by stable content ID and mapped onto the
/// first generated location slots. Their names, descriptions, danger state,
/// and authored exit relationships are preserved. Existing generated exits
/// remain, so the procedural graph continues to provide connectivity and
/// additional routes around authored locations.
pub fn place_authored_content(world: &mut World, content: &CampaignContent) -> usize {
    if world.regions.is_empty() || content.world.locations.is_empty() {
        return 0;
    }

    while world.locations.len() < content.world.locations.len() {
        add_generated_location(world);
    }

    let anchor_region_id = world.regions[0].id;
    world.regions[0].name = content.world.region.name.clone();
    world.regions[0].description = content.world.region.description.clone();

    let mut authored_locations = content.world.locations.iter().collect::<Vec<_>>();
    authored_locations.sort_by(|left, right| left.id.cmp(&right.id));

    let authored_runtime_ids = authored_locations
        .iter()
        .enumerate()
        .map(|(index, location)| (location.id.as_str(), world.locations[index].id))
        .collect::<std::collections::HashMap<_, _>>();

    for (index, authored) in authored_locations.iter().enumerate() {
        let generated_id = world.locations[index].id;
        let generated_exits = world.locations[index].exits.clone();
        let authored_exits = authored
            .exits
            .iter()
            .filter_map(|exit_id| authored_runtime_ids.get(exit_id.as_str()).copied())
            .collect::<Vec<_>>();

        let mut exits = generated_exits;
        for exit_id in authored_exits {
            if exit_id != generated_id && !exits.contains(&exit_id) {
                exits.push(exit_id);
            }
        }

        world.locations[index].name = authored.name.clone();
        world.locations[index].description = authored.description.clone();
        world.locations[index].dangerous = authored.dangerous;
        world.locations[index].region_id = anchor_region_id;
        world.locations[index].exits = exits;
        world.locations[index].spatial_layout = authored.spatial_layout.clone();
    }

    rebuild_region_location_ids(world);
    authored_locations.len()
}

fn add_generated_location(world: &mut World) {
    let id = world.allocate_id();
    let region_id = world
        .regions
        .last()
        .map(|region| region.id)
        .or_else(|| world.regions.first().map(|region| region.id))
        .expect("generated world has at least one region");
    let index = world.locations.len() + 1;
    world.locations.push(Location {
        id,
        name: format!("Generated Site {index:02}"),
        description: "An unexplored place shaped by the world seed.".to_string(),
        region_id,
        dangerous: false,
        corpse_ids: Vec::new(),
        exits: Vec::new(),
        spatial_layout: None,
    });
    if index > 1 {
        add_bidirectional_exit(world, index - 2, index - 1);
    }
}

fn rebuild_region_location_ids(world: &mut World) {
    for region in &mut world.regions {
        region.location_ids.clear();
    }
    for location in &world.locations {
        if let Some(region) = world
            .regions
            .iter_mut()
            .find(|region| region.id == location.region_id)
        {
            region.location_ids.push(location.id);
        }
    }
}

fn possible_extra_edges(location_count: usize) -> usize {
    location_count
        .saturating_mul(location_count.saturating_sub(1))
        .saturating_div(2)
        .saturating_sub(location_count.saturating_sub(1))
}

fn add_bidirectional_exit(world: &mut World, a_index: usize, b_index: usize) {
    let a_id = world.locations[a_index].id;
    let b_id = world.locations[b_index].id;
    if !has_exit(&world.locations[a_index], b_id) {
        world.locations[a_index].exits.push(b_id);
    }
    if !has_exit(&world.locations[b_index], a_id) {
        world.locations[b_index].exits.push(a_id);
    }
}

fn has_exit(location: &Location, target_id: u64) -> bool {
    location.exits.contains(&target_id)
}

#[cfg(test)]
#[path = "../tests/unit/procedural.rs"]
mod tests;
