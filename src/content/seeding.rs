use crate::content::definitions::CampaignContent;
use crate::model::{Location, Region, World};

impl CampaignContent {
    pub fn seed_world(&self, world: &mut World) -> usize {
        if world.regions.is_empty() {
            let region_id = world.allocate_id();
            world.regions.push(Region {
                id: region_id,
                name: self.world.region.name.clone(),
                description: self.world.region.description.clone(),
                location_ids: Vec::new(),
            });
        }

        let region_id = world
            .regions
            .first()
            .map(|region| region.id)
            .unwrap_or_else(|| {
                let id = world.allocate_id();
                world.regions.push(Region {
                    id,
                    name: self.world.region.name.clone(),
                    description: self.world.region.description.clone(),
                    location_ids: Vec::new(),
                });
                id
            });

        let mut added = 0usize;
        for location in &self.world.locations {
            if world.location_by_name(&location.name).is_none() {
                let id = world.allocate_id();
                world.locations.push(Location {
                    id,
                    name: location.name.clone(),
                    description: location.description.clone(),
                    region_id,
                    dangerous: location.dangerous,
                    corpse_ids: Vec::new(),
                    exits: Vec::new(),
                    spatial_layout: location.spatial_layout.clone(),
                });
                added += 1;
            }
        }

        for location in &self.world.locations.clone() {
            let exits = location
                .exits
                .iter()
                .filter_map(|exit_id| {
                    self.world
                        .locations
                        .iter()
                        .find(|candidate| candidate.id == *exit_id)
                })
                .filter_map(|exit_location| {
                    world
                        .location_by_name(&exit_location.name)
                        .map(|world_exit| world_exit.id)
                })
                .collect::<Vec<_>>();
            if let Some(world_location) = world.location_by_name_mut(&location.name) {
                world_location.description = location.description.clone();
                world_location.dangerous = location.dangerous;
                world_location.exits = exits;
                world_location.spatial_layout = location.spatial_layout.clone();
            }
        }

        if let Some(region) = world
            .regions
            .iter_mut()
            .find(|region| region.id == region_id)
        {
            region.name = self.world.region.name.clone();
            region.description = self.world.region.description.clone();
            region.location_ids = world
                .locations
                .iter()
                .filter(|location| location.region_id == region_id)
                .map(|location| location.id)
                .collect();
        }

        added
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::WorldMode;

    #[test]
    fn seed_world_adds_missing_content_locations_and_repairs_metadata() {
        let mut world = World::new("Test", WorldMode::New);
        let content = CampaignContent {
            version: 1,
            world: crate::content::definitions::WorldContent {
                region: crate::content::definitions::RegionContent {
                    id: "region.test".into(),
                    name: "Test Region".into(),
                    description: "Updated".into(),
                },
                locations: vec![crate::content::definitions::LocationContent {
                    id: "location.test".into(),
                    name: "Test Location".into(),
                    description: "Location".into(),
                    dangerous: true,
                    exits: vec![],
                    scene_art: None,
                    scene_image: None,
                    spatial_layout: Some("shrine_chamber".into()),
                }],
            },
            factions: vec![],
            npcs: vec![],
            quests: vec![],
            encounters: vec![],
            atmospheres: vec![],
            item_visuals: vec![],
            events: vec![],
            loaded_mods: vec![],
        };

        assert_eq!(content.seed_world(&mut world), 1);
        assert_eq!(world.locations.len(), 1);
        assert_eq!(world.locations[0].name, "Test Location");
        assert!(world.locations[0].dangerous);
        assert_eq!(
            world.locations[0].spatial_layout.as_deref(),
            Some("shrine_chamber")
        );
        assert_eq!(world.regions[0].name, "Test Region");
    }
}
