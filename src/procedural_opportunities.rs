use crate::content::{EventConditionContent, EventContent, EventEffectContent};
use crate::model::{EntityId, GameState, Quest, QuestObjective, QuestObjectiveKind};

const QUEST_PREFIX: &str = "generated.quest.";
const EVENT_PREFIX: &str = "generated.event.";
const EVOLUTION_QUEST_PREFIX: &str = "generated.evolution.quest.";
const EVOLUTION_EVENT_PREFIX: &str = "generated.evolution.event.";
const EVOLUTION_PROCESSED_EVENT_PREFIX: &str = "quest.evolution.processed.";
const RELATIONSHIP_MARKER: &str = "[generated relationship]";
const GENERATED_FACTION_MARKER: &str = "A generated faction shaped by";

pub fn populate_generated_opportunities(state: &mut GameState) -> usize {
    if state.world.generation.is_none() {
        return 0;
    }
    if let Some((faction_id, target_id, kind, location_id)) = select_relationship(state) {
        let mut added = create_relationship_quest(state, faction_id, target_id, &kind, location_id);
        let event_id = format!("{EVENT_PREFIX}{faction_id}.{target_id}.{kind}");
        if ensure_event(
            state,
            relationship_event(state, event_id, &kind, location_id),
        ) {
            added += 1;
        }
        if added > 0 {
            added
        } else {
            create_danger_opportunity(state)
        }
    } else {
        create_danger_opportunity(state)
    }
}

fn faction_member_npc(state: &GameState, faction_id: EntityId) -> Option<EntityId> {
    state
        .npcs
        .iter()
        .find(|npc| npc.faction_id == Some(faction_id))
        .map(|npc| npc.id)
}

fn create_relationship_quest(
    state: &mut GameState,
    faction_id: EntityId,
    target_id: EntityId,
    kind: &str,
    location_id: EntityId,
) -> usize {
    let id = format!("{QUEST_PREFIX}{faction_id}.{target_id}.{kind}");
    if state.quests.iter().any(|q| q.content_id == id) {
        return 0;
    }
    let location = state
        .world
        .location_by_id(location_id)
        .map(|l| l.name.clone())
        .unwrap_or_else(|| "an unknown place".into());
    let Some(giver) = faction_member_npc(state, faction_id) else {
        return 0;
    };
    let faction = state
        .factions
        .iter()
        .find(|f| f.id == faction_id)
        .map(|f| f.name.clone())
        .unwrap_or_else(|| "a local faction".into());
    let target = state
        .factions
        .iter()
        .find(|f| f.id == target_id)
        .map(|f| f.name.clone())
        .unwrap_or_else(|| "a rival faction".into());
    let title = match kind {
        "rivalry" => "Test the Rival's Reach",
        "alliance" => "Carry Word Between Allies",
        "trade" => "Secure the Trade Route",
        "influence" => "Gauge the Neighbour's Strength",
        "dependency" => "Trace the Missing Supply",
        _ => "Investigate a Faction's Reach",
    };
    let mut quest = Quest::new(
        state.world.allocate_id(),
        id,
        title,
        format!("{faction} has a {kind} relationship with {target}. Travel to {location} and learn what the relationship means on the ground."),
        location_id,
        faction_id,
        giver,
        "",
        format!("Token of {faction}"),
    );
    quest.objectives.push(QuestObjective::new(
        QuestObjectiveKind::VisitLocation,
        location,
        1,
    ));
    state.quests.push(quest);
    1
}

fn create_danger_opportunity(state: &mut GameState) -> usize {
    let Some(location) = state.world.locations.iter().find(|l| l.dangerous).cloned() else {
        return 0;
    };
    let Some(faction) = state
        .factions
        .iter()
        .find(|f| is_generated_faction(f))
        .cloned()
    else {
        return 0;
    };
    let Some(giver) = faction_member_npc(state, faction.id) else {
        return 0;
    };
    let id = format!("{QUEST_PREFIX}danger.{}", location.id);
    let mut added = 0;
    if !state.quests.iter().any(|q| q.content_id == id) {
        let mut quest = Quest::new(
            state.world.allocate_id(),
            id,
            "Scout the Dangerous Ground",
            format!(
                "The area around {} remains dangerous. Find out what is making it unsafe.",
                location.name
            ),
            location.id,
            faction.id,
            giver,
            "",
            format!("Scout's Token of {}", faction.name),
        );
        quest.objectives.push(QuestObjective::new(
            QuestObjectiveKind::VisitLocation,
            location.name.clone(),
            1,
        ));
        state.quests.push(quest);
        added += 1;
    }
    let event_id = format!("{EVENT_PREFIX}danger.{}", location.id);
    if ensure_event(
        state,
        EventContent {
            id: event_id,
            trigger: "travel_arrival".into(),
            weight: 1,
            chance_percent: Some(100),
            cooldown_turns: Some(12),
            conditions: Some(EventConditionContent {
                locations: vec![location.name.clone()],
                dangerous: Some(true),
                ..Default::default()
            }),
            effects: vec![EventEffectContent::History {
                text: format!(
                    "The danger around {} is close and immediate.",
                    location.name
                ),
            }],
        },
    ) {
        added += 1;
    }
    added
}

pub fn evolve_generated_world(state: &mut GameState) -> usize {
    if state.world.generation.is_none() {
        return 0;
    }
    let completed = state
        .quests
        .iter()
        .filter(|q| {
            q.completed
                && (q.content_id.starts_with(QUEST_PREFIX)
                    || q.content_id.starts_with(EVOLUTION_QUEST_PREFIX))
                && !evolution_already_processed(state, &q.content_id)
        })
        .map(|q| (q.content_id.clone(), q.target_location_id, q.faction_id))
        .collect::<Vec<_>>();
    let mut changed = 0;
    for (quest_id, location_id, faction_id) in completed {
        let Some(name) = pacify_location(state, location_id) else {
            continue;
        };
        changed += 1;
        state.world.record_event_history(
            state.character.turn,
            format!("{EVOLUTION_PROCESSED_EVENT_PREFIX}{quest_id}"),
            name.clone(),
            format!("The danger at {name} receded after {quest_id} was resolved."),
        );
        if let Some(faction) = state.factions.iter_mut().find(|f| f.id == faction_id) {
            faction.memory.push(format!(
                "[world evolution] {name} became safer after {quest_id} was completed."
            ));
        }
        create_follow_up(state, &quest_id, faction_id, location_id);
    }
    changed
}

fn evolution_already_processed(state: &GameState, quest_id: &str) -> bool {
    let event_id = format!("{EVOLUTION_PROCESSED_EVENT_PREFIX}{quest_id}");
    state
        .world
        .history
        .iter()
        .any(|entry| entry.event_id.as_deref() == Some(event_id.as_str()))
}

fn pacify_location(state: &mut GameState, location_id: EntityId) -> Option<String> {
    let location = state.world.location_by_id_mut(location_id)?;
    if !location.dangerous {
        return None;
    }
    location.dangerous = false;
    Some(location.name.clone())
}

fn create_follow_up(
    state: &mut GameState,
    source: &str,
    faction_id: EntityId,
    completed_location_id: EntityId,
) {
    let Some(target) = state
        .world
        .locations
        .iter()
        .find(|l| l.dangerous && l.id != completed_location_id)
        .cloned()
    else {
        return;
    };
    let Some(giver) = faction_member_npc(state, faction_id) else {
        return;
    };
    let id = format!("{EVOLUTION_QUEST_PREFIX}{source}");
    if state.quests.iter().any(|q| q.content_id == id) {
        return;
    }
    let faction = state
        .factions
        .iter()
        .find(|f| f.id == faction_id)
        .map(|f| f.name.clone())
        .unwrap_or_else(|| "the faction".into());
    let mut quest = Quest::new(
        state.world.allocate_id(),
        id,
        "Follow the Opening",
        format!(
            "With earlier ground safer, {faction} can redirect attention toward {}.",
            target.name
        ),
        target.id,
        faction_id,
        giver,
        "",
        format!("Renewed Token of {faction}"),
    );
    quest.objectives.push(QuestObjective::new(
        QuestObjectiveKind::VisitLocation,
        target.name.clone(),
        1,
    ));
    state.quests.push(quest);
    let event_id = format!("{EVOLUTION_EVENT_PREFIX}{source}");
    ensure_event(
        state,
        EventContent {
            id: event_id,
            trigger: "travel_arrival".into(),
            weight: 1,
            chance_percent: Some(100),
            cooldown_turns: Some(12),
            conditions: Some(EventConditionContent {
                locations: vec![target.name.clone()],
                dangerous: Some(true),
                ..Default::default()
            }),
            effects: vec![EventEffectContent::History {
                text: format!(
                    "With the earlier danger reduced, attention turns toward {}.",
                    target.name
                ),
            }],
        },
    );
}

fn relationship_event(
    state: &GameState,
    id: String,
    kind: &str,
    location_id: EntityId,
) -> EventContent {
    let location = state
        .world
        .location_by_id(location_id)
        .map(|l| l.name.clone())
        .unwrap_or_else(|| "Unknown".into());
    EventContent {
        id,
        trigger: "travel_arrival".into(),
        weight: 1,
        chance_percent: Some(100),
        cooldown_turns: Some(12),
        conditions: Some(EventConditionContent {
            locations: vec![location],
            ..Default::default()
        }),
        effects: vec![EventEffectContent::History {
            text: format!("The signs of the {kind} between the factions are unmistakable here."),
        }],
    }
}

fn ensure_event(state: &mut GameState, event: EventContent) -> bool {
    let Some(content) = state.campaign_content.as_mut() else {
        return false;
    };
    if content
        .events
        .iter()
        .any(|candidate| candidate.id == event.id)
    {
        return false;
    }
    content.events.push(event);
    true
}

fn select_relationship(state: &GameState) -> Option<(EntityId, EntityId, String, EntityId)> {
    let mut factions = state
        .factions
        .iter()
        .filter(|f| is_generated_faction(f))
        .collect::<Vec<_>>();
    factions.sort_by_key(|f| f.id);
    factions.into_iter().find_map(|faction| {
        let mut memories = faction
            .memory
            .iter()
            .filter(|m| m.starts_with(RELATIONSHIP_MARKER))
            .collect::<Vec<_>>();
        memories.sort();
        memories.into_iter().find_map(|memory| {
            let body = memory.strip_prefix(RELATIONSHIP_MARKER)?.trim_start();
            let (kind, rest) = body.split_once(" with ")?;
            let target_name = rest.split(" (strength").next()?.trim();
            let target = state.factions.iter().find(|f| f.name == target_name)?;
            let location = state
                .npcs
                .iter()
                .find(|npc| npc.faction_id == Some(target.id))
                .map(|npc| npc.location_id)
                .or_else(|| faction_location(state, target))
                .or_else(|| {
                    state
                        .world
                        .locations
                        .iter()
                        .find(|l| l.dangerous)
                        .map(|l| l.id)
                })?;
            Some((faction.id, target.id, kind.to_string(), location))
        })
    })
}

fn faction_location(state: &GameState, faction: &crate::model::Faction) -> Option<EntityId> {
    let region_name = faction.name.rsplit_once(" of ")?.1;
    let region_id = state
        .world
        .regions
        .iter()
        .find(|r| r.name == region_name)?
        .id;
    state
        .world
        .locations
        .iter()
        .find(|l| l.region_id == region_id)
        .map(|l| l.id)
}

fn is_generated_faction(faction: &crate::model::Faction) -> bool {
    faction
        .memory
        .iter()
        .any(|entry| entry.starts_with(GENERATED_FACTION_MARKER))
}

#[cfg(test)]
#[path = "../tests/unit/procedural_opportunities.rs"]
mod tests;
