use super::definitions::*;
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

impl ContentLoadReport {
    fn with_content(content: CampaignContent) -> Self {
        Self { content, loaded_mods: Vec::new(), warnings: Vec::new() }
    }
}

pub fn load_campaign_content() -> CampaignContent {
    load_campaign_content_report().content
}

pub fn load_campaign_content_report() -> ContentLoadReport {
    let mut report = ContentLoadReport::with_content(default_campaign_content());
    let installed = match crate::mod_manager::list_installed_mods() {
        Ok(installed) => installed,
        Err(error) => {
            report.warnings.push(format!("Could not inspect installed mods: {error}"));
            return report;
        }
    };

    for item in installed.into_iter().filter(|item| item.enabled) {
        let manifest = item.manifest.clone();
        // Its assets remain active even if its optional content file is invalid,
        // so record the dependency before attempting to merge campaign content.
        report.loaded_mods.push(manifest.clone());
        match load_mod_content(&item.directory, &manifest) {
            Ok(Some(mod_content)) => {
                merge_campaign_content(&mut report.content, mod_content, &mut report.warnings);
            }
            Ok(None) => {},
            Err(error) => report.warnings.push(format!(
                "could not load mod {} ({}) from {}: {}",
                manifest.id, manifest.name, item.directory.display(), error
            )),
        }
    }
    report.content.loaded_mods = report.loaded_mods.clone();
    report.warnings.extend(report.content.validate());
    report
}

fn load_content_file(path: &Path) -> io::Result<CampaignContent> {
    let data = fs::read(path)?;
    serde_json::from_slice(&data)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))
}

fn load_mod_content(mod_root: &Path, manifest: &ModManifest) -> io::Result<Option<CampaignContent>> {
    let Some(content_file) = manifest.content_file.as_deref() else {
        return Ok(None);
    };
    let relative = PathBuf::from(content_file);
    if relative.as_os_str().is_empty()
        || relative.is_absolute()
        || !relative.components().all(|component| matches!(component, Component::Normal(_)))
        || content_file.contains('\\')
    {
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "unsafe mod content path"));
    }
    let canonical_root = fs::canonicalize(mod_root)?;
    let canonical_path = fs::canonicalize(canonical_root.join(relative))?;
    if !canonical_path.starts_with(&canonical_root) || !canonical_path.is_file() {
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "mod content path escapes its directory"));
    }
    load_content_file(&canonical_path).map(Some)
}

fn merge_campaign_content(
    base: &mut CampaignContent,
    incoming: CampaignContent,
    warnings: &mut Vec<String>,
) {
    base.world.region = incoming.world.region;
    merge_vec_by_key(
        &mut base.world.locations,
        incoming.world.locations,
        |entry| entry.id.clone(),
    );
    merge_vec_by_key(&mut base.factions, incoming.factions, |entry| {
        entry.id.clone()
    });
    merge_vec_by_key(&mut base.npcs, incoming.npcs, |entry| entry.id.clone());
    merge_vec_by_key(&mut base.quests, incoming.quests, |entry| entry.id.clone());
    merge_vec_by_key(&mut base.encounters, incoming.encounters, |entry| {
        entry.location_name.clone()
    });
    merge_vec_by_key(&mut base.atmospheres, incoming.atmospheres, |entry| {
        entry.location_name.clone()
    });
    merge_vec_by_key(&mut base.item_visuals, incoming.item_visuals, |entry| {
        entry.item_name.clone()
    });

    let locations: HashSet<&str> = base
        .world
        .locations
        .iter()
        .map(|entry| entry.name.as_str())
        .collect();
    let factions: HashSet<&str> = base
        .factions
        .iter()
        .map(|entry| entry.name.as_str())
        .collect();
    let existing: HashSet<String> = base.events.iter().map(|event| event.id.clone()).collect();
    let known: HashSet<String> = existing
        .iter()
        .cloned()
        .chain(incoming.events.iter().map(|event| event.id.clone()))
        .collect();
    let accepted = filter_valid_events(
        incoming.events,
        &locations,
        &factions,
        &known,
        &existing,
        "mod content",
        warnings,
    );
    base.events.extend(accepted);
}

fn filter_valid_events(
    events: Vec<EventContent>,
    locations: &HashSet<&str>,
    factions: &HashSet<&str>,
    known_ids: &HashSet<String>,
    existing_ids: &HashSet<String>,
    source: &str,
    warnings: &mut Vec<String>,
) -> Vec<EventContent> {
    let mut accepted = Vec::new();
    let mut seen_ids = existing_ids.clone();
    let mut pending = Vec::new();
    for event in events {
        let mut issues = validate_event_content(&event, locations, factions, known_ids);
        if !seen_ids.insert(event.id.clone()) {
            issues.push(format!("duplicate event id {}", event.id));
        }
        if issues.is_empty() {
            pending.push(event);
        } else {
            warnings.push(format!(
                "rejecting event '{}' from {}: {}",
                event.id,
                source,
                issues.join("; ")
            ));
        }
    }
    loop {
        let available: HashSet<String> = existing_ids
            .iter()
            .cloned()
            .chain(accepted.iter().map(|event: &EventContent| event.id.clone()))
            .chain(pending.iter().map(|event| event.id.clone()))
            .collect();
        let mut removed = false;
        let mut next = Vec::new();
        for event in pending {
            let prior = event
                .conditions
                .as_ref()
                .and_then(|conditions| conditions.prior_event_id.as_deref());
            if let Some(prior_id) = prior.filter(|id| !available.contains(*id)) {
                warnings.push(format!(
                    "rejecting event '{}' from {}: prior event '{}' was rejected or unavailable",
                    event.id, source, prior_id
                ));
                removed = true;
            } else {
                next.push(event);
            }
        }
        pending = next;
        if !removed {
            break;
        }
    }
    accepted.extend(pending);
    accepted
}

fn validate_event_content(
    event: &EventContent,
    locations: &HashSet<&str>,
    factions: &HashSet<&str>,
    known_ids: &HashSet<String>,
) -> Vec<String> {
    let mut issues = Vec::new();
    if event.id.trim().is_empty() {
        issues.push("empty id".into());
    }
    if event.trigger.trim().is_empty() {
        issues.push("empty trigger".into());
    }
    if event.weight == 0 {
        issues.push("zero weight".into());
    }
    if let Some(chance) = event.chance_percent {
        if chance > 100 {
            issues.push(format!("invalid chance {}", chance));
        }
    }
    if event.effects.is_empty() {
        issues.push("no effects".into());
    }
    if let Some(conditions) = &event.conditions {
        for location in &conditions.locations {
            if !locations.contains(location.as_str()) {
                issues.push(format!("unknown location {}", location));
            }
        }
        if let (Some(min_day), Some(max_day)) = (conditions.min_day, conditions.max_day) {
            if min_day > max_day {
                issues.push("min_day greater than max_day".into());
            }
        }
        if let Some(prior) = conditions.prior_event_id.as_deref() {
            if !known_ids.contains(prior) {
                issues.push(format!("unknown prior event id {}", prior));
            }
        }
        if let Some(faction) = conditions.faction_name.as_deref() {
            if !factions.contains(faction) {
                issues.push(format!("unknown faction {}", faction));
            }
        }
        if (conditions.min_reputation.is_some() || conditions.max_reputation.is_some())
            && conditions.faction_name.is_none()
        {
            issues.push("reputation condition requires faction_name".into());
        }
        if let (Some(min), Some(max)) = (conditions.min_reputation, conditions.max_reputation) {
            if min > max {
                issues.push("min_reputation greater than max_reputation".into());
            }
        }
        if conditions
            .required_item_name
            .as_deref()
            .map(|name| name.trim().is_empty())
            .unwrap_or(false)
        {
            issues.push("required_item_name cannot be empty".into());
        }
        if conditions
            .required_condition_name
            .as_deref()
            .map(|name| name.trim().is_empty())
            .unwrap_or(false)
        {
            issues.push("required_condition_name cannot be empty".into());
        }
    }
    issues
}

fn merge_vec_by_key<T, F>(base: &mut Vec<T>, incoming: Vec<T>, key_fn: F)
where
    T: Clone,
    F: Fn(&T) -> String,
{
    for item in incoming {
        let key = key_fn(&item);
        if let Some(position) = base.iter().position(|existing| key_fn(existing) == key) {
            base[position] = item;
        } else {
            base.push(item);
        }
    }
}

fn default_campaign_content() -> CampaignContent {
    serde_json::from_str(include_str!("../../data/base_content.json"))
        .expect("embedded base content JSON must remain valid")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_event(id: &str) -> EventContent {
        EventContent {
            id: id.into(),
            trigger: "travel_arrival".into(),
            weight: 1,
            chance_percent: Some(100),
            cooldown_turns: None,
            conditions: None,
            effects: vec![EventEffectContent::Pause],
        }
    }

    #[test]
    fn invalid_events_are_rejected_and_reported() {
        let mut warnings = Vec::new();
        let locations = HashSet::from(["Ashen Gate"]);
        let factions = HashSet::from(["Cinder Wardens"]);
        let known = HashSet::from(["good.event".into(), "bad.event".into()]);
        let mut invalid = valid_event("bad.event");
        invalid.weight = 0;
        invalid.conditions = Some(EventConditionContent {
            locations: vec!["Unknown Place".into()],
            ..Default::default()
        });
        let accepted = filter_valid_events(
            vec![valid_event("good.event"), invalid],
            &locations,
            &factions,
            &known,
            &HashSet::new(),
            "test content",
            &mut warnings,
        );
        assert_eq!(accepted.len(), 1);
        assert_eq!(accepted[0].id, "good.event");
        assert!(warnings[0].contains("zero weight"));
        assert!(warnings[0].contains("unknown location"));
    }
}
