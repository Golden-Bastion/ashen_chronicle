use crate::game::{character, interactions, state_effects};
use crate::model::{Corpse, GameState, Item};
use crate::presentation::{ItemView, RemainsEntryView, RemainsResultView, RemainsView};
use std::mem;
use std::time::{SystemTime, UNIX_EPOCH};

fn item_view(item: &Item) -> ItemView {
    ItemView {
        id: item.id,
        name: item.name.clone(),
        description: item.description.clone(),
    }
}

pub(crate) fn build_remains_view_for_bevy(state: &GameState) -> RemainsView {
    let location_id = state.character.location_id;
    let location_name = state
        .world
        .location_by_id(location_id)
        .map(|location| location.name.clone())
        .unwrap_or_else(|| "this place".to_string());
    let remains = state
        .corpses
        .iter()
        .filter(|corpse| corpse.location_id == location_id && !corpse.inventory.is_empty())
        .map(|corpse| RemainsEntryView {
            id: corpse.id,
            label: corpse_label(corpse),
            former_name: corpse.former_name.clone(),
            former_title: corpse.former_title.clone(),
            scavenged: corpse.scavenged,
            items: corpse.inventory.iter().map(item_view).collect(),
        })
        .collect();
    RemainsView {
        location_name,
        remains,
    }
}

pub(crate) fn search_remains_for_bevy(
    state: &mut GameState,
    selection: usize,
) -> Result<RemainsResultView, String> {
    let location_id = state.character.location_id;
    let indices: Vec<usize> = state
        .corpses
        .iter()
        .enumerate()
        .filter(|(_, corpse)| corpse.location_id == location_id && !corpse.inventory.is_empty())
        .map(|(index, _)| index)
        .collect();
    let corpse_index = *indices
        .get(selection)
        .ok_or_else(|| "Those remains are no longer available.".to_string())?;
    let location_name = state
        .world
        .location_by_id(location_id)
        .map(|location| location.name.clone())
        .unwrap_or_else(|| "this place".to_string());

    let (former_name, former_title, items, corpse_id) = {
        let corpse = &mut state.corpses[corpse_index];
        let items = mem::take(&mut corpse.inventory);
        corpse.scavenged = true;
        (
            corpse.former_name.clone(),
            corpse.former_title.clone(),
            items,
            corpse.id,
        )
    };
    state_effects::advance_time(state, 1);
    state.character.turn += 1;

    if items.is_empty() {
        state.world.record_history(
            state.character.turn,
            format!(
                "{} searched the remains of {} the {} at {}.",
                state.character.display_name(),
                former_name,
                former_title,
                location_name
            ),
        );
        return Err("Nothing useful remains.".to_string());
    }

    let item_names: Vec<String> = items.iter().map(|item| item.name.clone()).collect();
    let recovered_items = items.iter().map(item_view).collect::<Vec<_>>();
    for item in &items {
        interactions::grant_reward_reputation(state, item);
    }
    state.character.inventory.extend(items);

    let hidden_item = if state.character.effective_insight() >= 2 && item_names.len() < 3 {
        let tick = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0);
        if tick.is_multiple_of(2) {
            let hidden = Item {
                id: state.world.allocate_id(),
                name: "Ashen Note".to_string(),
                description:
                    "A scrap of writing that might reveal something about the life that ended here."
                        .to_string(),
            };
            state.character.inventory.push(hidden.clone());
            Some(hidden)
        } else {
            None
        }
    } else {
        None
    };

    character::gain_experience(
        state,
        remains_experience(state.character.effective_insight()),
    );

    let result_view = RemainsResultView {
        location_name: location_name.clone(),
        former_name: former_name.clone(),
        former_title: former_title.clone(),
        items: recovered_items,
        hidden_item: hidden_item.as_ref().map(item_view),
        notes: vec![
            "Feel like a deja-vu.".to_string(),
            "You feel as if they were once yours. Though, These items can be inherited, Their memories cannot.".to_string(),
            format!("Recovered {}", item_names.join(", ")),
        ],
    };
    state.world.record_history(
        state.character.turn,
        format!(
            "{} searched the remains of {} the {} at {}.",
            state.character.display_name(),
            former_name,
            former_title,
            location_name
        ),
    );
    if let Some(location) = state.world.location_by_id_mut(location_id) {
        if !location.corpse_ids.contains(&corpse_id) {
            location.corpse_ids.push(corpse_id);
        }
    }
    Ok(result_view)
}

fn remains_experience(effective_insight: i32) -> u32 {
    (5 + effective_insight).max(0) as u32
}

fn corpse_label(corpse: &Corpse) -> String {
    if corpse.former_name.is_empty() {
        "Unidentified remains".to_string()
    } else if corpse.scavenged {
        format!(
            "{} the {} (searched)",
            corpse.former_name, corpse.former_title
        )
    } else {
        format!("{} the {}", corpse.former_name, corpse.former_title)
    }
}

pub(crate) fn mark_character_dead(state: &mut GameState, cause: String, location_name: &str) {
    if !state.character.alive {
        return;
    }
    state.character.alive = false;
    state.character.hp = 0;
    let corpse = create_corpse(state, cause.clone());
    state.corpses.push(corpse.clone());
    if let Some(location) = state.world.location_by_id_mut(corpse.location_id) {
        if !location.corpse_ids.contains(&corpse.id) {
            location.corpse_ids.push(corpse.id);
        }
    }
    let character_name = state.character.display_name();
    state.world.record_history(
        state.character.turn,
        format!("{} died at {} ({cause}).", character_name, location_name),
    );
    interactions::update_faction_memory_for_location(
        state,
        corpse.location_id,
        format!("{} died at {}.", character_name, location_name),
    );
}

fn create_corpse(state: &mut GameState, epitaph: String) -> Corpse {
    let corpse_id = state.world.allocate_id();
    let location_id = state.character.location_id;
    let inventory = mem::take(&mut state.character.inventory);
    Corpse {
        id: corpse_id,
        former_name: state.character.name.clone(),
        former_title: state.character.title.clone(),
        location_id,
        turn_of_death: state.character.turn,
        inventory,
        epitaph,
        scavenged: false,
    }
}

#[cfg(test)]
#[path = "../../tests/unit/game/legacy.rs"]
mod tests;
