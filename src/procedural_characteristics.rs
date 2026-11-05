use crate::model::{EntityId, World};
use crate::rng::DeterministicRng;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionTheme {
    Frontier,
    Woodland,
    Highlands,
    Marsh,
    Wastes,
    Coast,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Climate {
    Cold,
    Temperate,
    Arid,
    Wet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocationKind {
    Settlement,
    Ruin,
    Wilderness,
    Mine,
    Shrine,
    Crossroads,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegionCharacteristics {
    pub region_id: EntityId,
    pub theme: RegionTheme,
    pub climate: Climate,
    pub prosperity: u8,
    pub danger: u8,
    pub population: u32,
    pub resources: Vec<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocationCharacteristics {
    pub location_id: EntityId,
    pub region_id: EntityId,
    pub kind: LocationKind,
    pub danger: u8,
    pub population: u32,
    pub resources: Vec<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldCharacteristics {
    pub regions: Vec<RegionCharacteristics>,
    pub locations: Vec<LocationCharacteristics>,
}

/// Derive deterministic world characteristics from the persisted world seed
/// and the generated region/location identities.
///
/// These characteristics are intentionally derived rather than stored as a
/// second copy of world state. Rebuilding them from the same generated world
/// therefore produces the same result after save/load without regenerating
/// gameplay state.
pub fn generate_world_characteristics(world: &World) -> WorldCharacteristics {
    let seed = world
        .generation
        .as_ref()
        .map(|generation| generation.seed)
        .unwrap_or(world.id);

    let regions = world
        .regions
        .iter()
        .enumerate()
        .map(|(index, region)| generate_region_characteristics(seed, index, region.id))
        .collect::<Vec<_>>();

    let locations = world
        .locations
        .iter()
        .enumerate()
        .map(|(index, location)| {
            let region = regions
                .iter()
                .find(|region| region.region_id == location.region_id)
                .expect("every generated location belongs to a generated region");
            generate_location_characteristics(seed, index, location.id, region)
        })
        .collect::<Vec<_>>();

    WorldCharacteristics { regions, locations }
}

fn generate_region_characteristics(
    seed: u64,
    index: usize,
    region_id: EntityId,
) -> RegionCharacteristics {
    let mut rng = DeterministicRng::new(seed ^ region_id.rotate_left(17) ^ index as u64);
    let theme = match rng.gen_range(0..6) {
        0 => RegionTheme::Frontier,
        1 => RegionTheme::Woodland,
        2 => RegionTheme::Highlands,
        3 => RegionTheme::Marsh,
        4 => RegionTheme::Wastes,
        _ => RegionTheme::Coast,
    };
    let climate = climate_for(theme, &mut rng);
    let prosperity = base_prosperity(theme).saturating_add(rng.gen_range(0..21) as u8);
    let danger = base_danger(theme).saturating_add(rng.gen_range(0..21) as u8);
    let population = 80 + rng.gen_range(0..421) as u32 + (prosperity as u32 * 4);
    let resources = region_resources(theme, climate);
    let tags = region_tags(theme, climate, prosperity, danger);

    RegionCharacteristics {
        region_id,
        theme,
        climate,
        prosperity,
        danger,
        population,
        resources,
        tags,
    }
}

fn generate_location_characteristics(
    seed: u64,
    index: usize,
    location_id: EntityId,
    region: &RegionCharacteristics,
) -> LocationCharacteristics {
    let mut rng = DeterministicRng::new(
        seed ^ location_id.rotate_left(29) ^ (index as u64).wrapping_mul(0xD6E8_FEB8_6659_FD93),
    );
    let kind = location_kind_for(region.theme, region.prosperity, region.danger, &mut rng);
    let danger = region
        .danger
        .saturating_add(match kind {
            LocationKind::Settlement => 0,
            LocationKind::Ruin => 12,
            LocationKind::Wilderness => 8,
            LocationKind::Mine => 4,
            LocationKind::Shrine => 6,
            LocationKind::Crossroads => 2,
        })
        .min(100);
    let population = match kind {
        LocationKind::Settlement => 20 + rng.gen_range(0..181) as u32,
        LocationKind::Crossroads => 10 + rng.gen_range(0..81) as u32,
        LocationKind::Mine => 8 + rng.gen_range(0..73) as u32,
        LocationKind::Shrine => rng.gen_range(0..31) as u32,
        LocationKind::Ruin | LocationKind::Wilderness => rng.gen_range(0..11) as u32,
    };
    let resources = location_resources(kind, &region.resources);
    let tags = location_tags(kind, danger, population);

    LocationCharacteristics {
        location_id,
        region_id: region.region_id,
        kind,
        danger,
        population,
        resources,
        tags,
    }
}

fn climate_for(theme: RegionTheme, rng: &mut DeterministicRng) -> Climate {
    match theme {
        RegionTheme::Highlands | RegionTheme::Wastes => {
            if rng.gen_range(0..4) == 0 {
                Climate::Arid
            } else {
                Climate::Cold
            }
        }
        RegionTheme::Marsh | RegionTheme::Coast => Climate::Wet,
        RegionTheme::Woodland | RegionTheme::Frontier => {
            if rng.gen_range(0..5) == 0 {
                Climate::Cold
            } else {
                Climate::Temperate
            }
        }
    }
}

fn base_prosperity(theme: RegionTheme) -> u8 {
    match theme {
        RegionTheme::Frontier => 35,
        RegionTheme::Woodland => 45,
        RegionTheme::Highlands => 40,
        RegionTheme::Marsh => 30,
        RegionTheme::Wastes => 15,
        RegionTheme::Coast => 55,
    }
}

fn base_danger(theme: RegionTheme) -> u8 {
    match theme {
        RegionTheme::Frontier => 45,
        RegionTheme::Woodland => 35,
        RegionTheme::Highlands => 50,
        RegionTheme::Marsh => 60,
        RegionTheme::Wastes => 70,
        RegionTheme::Coast => 30,
    }
}

fn region_resources(theme: RegionTheme, climate: Climate) -> Vec<String> {
    let mut resources = match theme {
        RegionTheme::Frontier => vec!["timber".to_string(), "game".to_string()],
        RegionTheme::Woodland => vec!["timber".to_string(), "herbs".to_string()],
        RegionTheme::Highlands => vec!["ore".to_string(), "stone".to_string()],
        RegionTheme::Marsh => vec!["reeds".to_string(), "herbs".to_string()],
        RegionTheme::Wastes => vec!["scrap".to_string(), "salt".to_string()],
        RegionTheme::Coast => vec!["fish".to_string(), "salt".to_string()],
    };

    if matches!(climate, Climate::Wet) && !resources.iter().any(|resource| resource == "water") {
        resources.push("water".to_string());
    }
    resources
}

fn region_tags(theme: RegionTheme, climate: Climate, prosperity: u8, danger: u8) -> Vec<String> {
    let mut tags = vec![
        theme_tag(theme).to_string(),
        climate_tag(climate).to_string(),
    ];
    if prosperity >= 65 {
        tags.push("prosperous".to_string());
    } else if prosperity <= 30 {
        tags.push("impoverished".to_string());
    }
    if danger >= 65 {
        tags.push("perilous".to_string());
    } else if danger <= 30 {
        tags.push("stable".to_string());
    }
    tags
}

fn location_kind_for(
    theme: RegionTheme,
    prosperity: u8,
    danger: u8,
    rng: &mut DeterministicRng,
) -> LocationKind {
    let roll = rng.gen_range(0..100);
    if prosperity >= 60 && roll < 35 {
        return LocationKind::Settlement;
    }
    if danger >= 65 && roll < 35 {
        return LocationKind::Wilderness;
    }
    match theme {
        RegionTheme::Highlands if roll < 45 => LocationKind::Mine,
        RegionTheme::Woodland if roll < 55 => LocationKind::Shrine,
        RegionTheme::Wastes if roll < 45 => LocationKind::Ruin,
        RegionTheme::Coast if roll < 40 => LocationKind::Crossroads,
        RegionTheme::Marsh if roll < 45 => LocationKind::Wilderness,
        _ if roll < 25 => LocationKind::Ruin,
        _ if roll < 50 => LocationKind::Wilderness,
        _ if roll < 75 => LocationKind::Crossroads,
        _ => LocationKind::Settlement,
    }
}

fn location_resources(kind: LocationKind, region_resources: &[String]) -> Vec<String> {
    let mut resources = region_resources.iter().take(2).cloned().collect::<Vec<_>>();
    match kind {
        LocationKind::Mine => resources.push("ore".to_string()),
        LocationKind::Settlement => resources.push("supplies".to_string()),
        LocationKind::Shrine => resources.push("relics".to_string()),
        LocationKind::Ruin => resources.push("artifacts".to_string()),
        LocationKind::Wilderness | LocationKind::Crossroads => {}
    }
    resources.sort();
    resources.dedup();
    resources
}

fn location_tags(kind: LocationKind, danger: u8, population: u32) -> Vec<String> {
    let mut tags = vec![kind_tag(kind).to_string()];
    if danger >= 65 {
        tags.push("dangerous".to_string());
    }
    if population >= 100 {
        tags.push("populated".to_string());
    }
    tags
}

fn theme_tag(theme: RegionTheme) -> &'static str {
    match theme {
        RegionTheme::Frontier => "frontier",
        RegionTheme::Woodland => "woodland",
        RegionTheme::Highlands => "highlands",
        RegionTheme::Marsh => "marsh",
        RegionTheme::Wastes => "wastes",
        RegionTheme::Coast => "coast",
    }
}

fn climate_tag(climate: Climate) -> &'static str {
    match climate {
        Climate::Cold => "cold",
        Climate::Temperate => "temperate",
        Climate::Arid => "arid",
        Climate::Wet => "wet",
    }
}

fn kind_tag(kind: LocationKind) -> &'static str {
    match kind {
        LocationKind::Settlement => "settlement",
        LocationKind::Ruin => "ruin",
        LocationKind::Wilderness => "wilderness",
        LocationKind::Mine => "mine",
        LocationKind::Shrine => "shrine",
        LocationKind::Crossroads => "crossroads",
    }
}

#[cfg(test)]
#[path = "../tests/unit/procedural_characteristics.rs"]
mod tests;
