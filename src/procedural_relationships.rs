use crate::model::{Faction, GameState};
use crate::procedural_authored::authored_anchor_region;
use crate::procedural_characteristics::{generate_world_characteristics, RegionCharacteristics};

const RELATIONSHIP_MARKER: &str = "[generated relationship]";

/// Add deterministic relationships between generated factions and between
/// generated factions and authored factions using the characteristics of the
/// regions they inhabit. Relationships are stored in faction memory so they
/// persist using the existing runtime model and saves.
pub fn populate_generated_relationships(state: &mut GameState) -> usize {
    if state.world.generation.is_none() {
        return 0;
    }

    let characteristics = generate_world_characteristics(&state.world);
    let mut factions = state
        .factions
        .iter()
        .filter_map(|faction| {
            let generated = is_generated_faction(faction);
            let region = if generated {
                faction_region(faction, &state.world.regions, &characteristics).cloned()
            } else {
                authored_anchor_region(
                    &faction.name,
                    &faction.memory,
                    &state.world.regions,
                    &characteristics,
                )
            }?;
            Some((faction.id, region, generated))
        })
        .collect::<Vec<_>>();
    factions.sort_by_key(|(faction_id, _, _)| *faction_id);

    let mut added = 0;
    for left_index in 0..factions.len() {
        for right_index in left_index + 1..factions.len() {
            let (left_id, left_region, left_generated) = &factions[left_index];
            let (right_id, right_region, right_generated) = &factions[right_index];
            if !left_generated && !right_generated {
                continue;
            }
            let Some((kind, strength, reason)) = relationship_for(left_region, right_region) else {
                continue;
            };

            let left_name = state
                .factions
                .iter()
                .find(|faction| faction.id == *left_id)
                .map(|faction| faction.name.clone())
                .unwrap_or_default();
            let right_name = state
                .factions
                .iter()
                .find(|faction| faction.id == *right_id)
                .map(|faction| faction.name.clone())
                .unwrap_or_default();

            let left_memory = format!(
                "{RELATIONSHIP_MARKER} {kind} with {right_name} (strength {strength}): {reason}"
            );
            let right_memory = format!(
                "{RELATIONSHIP_MARKER} {kind} with {left_name} (strength {strength}): {reason}"
            );
            let already_present = state
                .factions
                .iter()
                .find(|faction| faction.id == *left_id)
                .map(|faction| faction.memory.iter().any(|entry| entry == &left_memory))
                .unwrap_or(true);
            if already_present {
                continue;
            }

            if let Some(faction) = state
                .factions
                .iter_mut()
                .find(|faction| faction.id == *left_id)
            {
                faction.memory.push(left_memory);
            }
            if let Some(faction) = state
                .factions
                .iter_mut()
                .find(|faction| faction.id == *right_id)
            {
                faction.memory.push(right_memory);
            }
            added += 1;
        }
    }

    added
}

fn is_generated_faction(faction: &Faction) -> bool {
    faction
        .memory
        .iter()
        .any(|entry| entry.starts_with("A generated faction shaped by"))
}

fn faction_region<'a>(
    faction: &Faction,
    world_regions: &'a [crate::model::Region],
    characteristics: &'a crate::procedural_characteristics::WorldCharacteristics,
) -> Option<&'a RegionCharacteristics> {
    let world_region = world_regions
        .iter()
        .find(|region| faction.name.ends_with(&format!("of {}", region.name)))?;
    characteristics
        .regions
        .iter()
        .find(|region| region.region_id == world_region.id)
}

fn relationship_for(
    left: &RegionCharacteristics,
    right: &RegionCharacteristics,
) -> Option<(&'static str, u8, String)> {
    let shared_resource = left
        .resources
        .iter()
        .find(|resource| right.resources.contains(resource));
    if let Some(resource) = shared_resource {
        let strength = 45 + left.danger.max(right.danger) / 2;
        return Some((
            "rivalry",
            strength,
            format!(
                "both regions depend on {resource}, and the surrounding danger increases competition"
            ),
        ));
    }

    if left.danger >= 60 && right.danger >= 60 {
        return Some((
            "alliance",
            70,
            "both regions face severe danger, creating pressure to cooperate for mutual protection"
                .to_string(),
        ));
    }

    if complementary_resources(left, right) {
        return Some((
            "trade",
            55,
            format!(
                "{} resources complement {} resources across the two regions",
                left.resources.join(", "),
                right.resources.join(", ")
            ),
        ));
    }

    let prosperity_gap = left.prosperity.abs_diff(right.prosperity);
    if prosperity_gap >= 30 {
        return Some((
            "influence",
            40 + prosperity_gap.min(50),
            "the more prosperous region has greater leverage over its less prosperous neighbour"
                .to_string(),
        ));
    }

    if left.climate != right.climate && left.theme != right.theme {
        return Some((
            "dependency",
            35,
            "different environments create a practical need for resources and services from the other region"
                .to_string(),
        ));
    }

    None
}

fn complementary_resources(left: &RegionCharacteristics, right: &RegionCharacteristics) -> bool {
    left.resources.iter().any(|left_resource| {
        right.resources.iter().any(|right_resource| {
            matches!(
                (left_resource.as_str(), right_resource.as_str()),
                ("fish", "timber")
                    | ("timber", "fish")
                    | ("fish", "ore")
                    | ("ore", "fish")
                    | ("timber", "ore")
                    | ("ore", "timber")
                    | ("herbs", "ore")
                    | ("ore", "herbs")
            )
        })
    })
}

#[cfg(test)]
#[path = "../tests/unit/procedural_relationships.rs"]
mod tests;
