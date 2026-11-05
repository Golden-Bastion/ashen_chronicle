use crate::content::CampaignContent;
use crate::model::{GameState, Region};
use crate::procedural_characteristics::{RegionCharacteristics, WorldCharacteristics};

const AUTHORED_ANCHOR_MARKER: &str = "[authored anchor]";

/// Integrate authored campaign entities with the generated world without
/// replacing their canonical names, locations, faction assignments, or quest
/// references. Anchor information is stored in existing faction/NPC memory
/// so it persists with the normal save data.
pub fn integrate_authored_content(state: &mut GameState, content: &CampaignContent) -> usize {
    if state.world.generation.is_none() {
        return 0;
    }

    let mut added = 0;

    for npc_content in &content.npcs {
        let Some(npc_index) = state
            .npcs
            .iter()
            .position(|npc| npc.name == npc_content.name)
        else {
            continue;
        };
        let npc = &state.npcs[npc_index];
        let Some(location) = state.world.location_by_id(npc.location_id) else {
            continue;
        };
        let Some(region) = state
            .world
            .regions
            .iter()
            .find(|region| region.id == location.region_id)
        else {
            continue;
        };
        let marker = format!(
            "{AUTHORED_ANCHOR_MARKER} {} operates at {} in {}.",
            npc.name, location.name, region.name
        );
        if !state.npcs[npc_index]
            .memory
            .iter()
            .any(|entry| entry == &marker)
        {
            state.npcs[npc_index].memory.push(marker);
            added += 1;
        }
    }

    let authored_faction_regions = content
        .factions
        .iter()
        .filter_map(|faction| {
            content
                .npcs
                .iter()
                .filter(|npc| npc.faction_name.as_deref() == Some(faction.name.as_str()))
                .filter_map(|npc| {
                    state
                        .npcs
                        .iter()
                        .find(|candidate| candidate.name == npc.name)
                })
                .filter_map(|npc| state.world.location_by_id(npc.location_id))
                .filter_map(|location| {
                    state
                        .world
                        .regions
                        .iter()
                        .find(|region| region.id == location.region_id)
                })
                .min_by_key(|region| region.id)
                .map(|region| (faction.name.as_str(), region.name.as_str()))
        })
        .collect::<Vec<_>>();

    for (faction_name, region_name) in authored_faction_regions {
        let marker =
            format!("{AUTHORED_ANCHOR_MARKER} {faction_name} is anchored in {region_name}.");
        let Some(faction) = state
            .factions
            .iter_mut()
            .find(|faction| faction.name == faction_name)
        else {
            continue;
        };
        if !faction.memory.iter().any(|entry| entry == &marker) {
            faction.memory.push(marker);
            added += 1;
        }
    }

    added
}

pub(crate) fn authored_anchor_region(
    faction_name: &str,
    faction_memory: &[String],
    world_regions: &[Region],
    characteristics: &WorldCharacteristics,
) -> Option<RegionCharacteristics> {
    let prefix = format!("{AUTHORED_ANCHOR_MARKER} {faction_name} is anchored in ");
    let region_name = faction_memory
        .iter()
        .find_map(|entry| entry.strip_prefix(&prefix))
        .map(|value| value.trim_end_matches('.'))?;
    let region = world_regions
        .iter()
        .find(|region| region.name == region_name)?;
    characteristics
        .regions
        .iter()
        .find(|characteristics| characteristics.region_id == region.id)
        .cloned()
}

#[cfg(test)]
#[path = "../tests/unit/procedural_authored.rs"]
mod tests;
