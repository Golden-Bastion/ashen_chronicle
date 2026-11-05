use crate::content::{EventConditionContent, EventContent, EventEffectContent};
use crate::model::{Condition, GameState};
use crate::rng::DeterministicRng;

#[derive(Debug, Clone)]
pub struct EventContext<'a> {
    pub trigger: &'a str,
    pub location_name: &'a str,
    pub dangerous: bool,
    pub night: bool,
}

impl<'a> EventContext<'a> {
    pub fn for_travel_arrival(location_name: &'a str, dangerous: bool, night: bool) -> Self {
        Self {
            trigger: "travel_arrival",
            location_name,
            dangerous,
            night,
        }
    }
}

pub fn trigger_event(state: &mut GameState, context: &EventContext<'_>) -> bool {
    let chance_roll = next_random_roll(state) % 100;
    let candidate_indices = {
        let Some(events) = state
            .campaign_content
            .as_ref()
            .map(|content| &content.events)
        else {
            return false;
        };
        events
            .iter()
            .enumerate()
            .filter(|(_, event)| event.trigger == context.trigger)
            .filter(|(_, event)| matches_conditions(event.conditions.as_ref(), state, context))
            .filter(|(_, event)| event_is_off_cooldown(state, &event.id))
            .filter(|(_, event)| event.chance_percent.unwrap_or(100) as u64 > chance_roll)
            .map(|(index, _)| index)
            .collect::<Vec<_>>()
    };

    if candidate_indices.is_empty() {
        return false;
    }

    let weight_roll = next_random_roll(state);
    let chosen_index = {
        let Some(events) = state
            .campaign_content
            .as_ref()
            .map(|content| &content.events)
        else {
            return false;
        };
        let total_weight = candidate_indices
            .iter()
            .map(|&index| events[index].weight.max(1) as u64)
            .sum::<u64>();
        let mut cursor = weight_roll % total_weight;
        let mut chosen = candidate_indices[0];
        for &index in &candidate_indices {
            let weight = events[index].weight.max(1) as u64;
            if cursor < weight {
                chosen = index;
                break;
            }
            cursor -= weight;
        }
        chosen
    };

    let Some(chosen) = state
        .campaign_content
        .as_ref()
        .and_then(|content| content.events.get(chosen_index))
        .cloned()
    else {
        return false;
    };
    apply_event(state, &chosen, context);
    true
}

fn matches_conditions(
    conditions: Option<&EventConditionContent>,
    state: &GameState,
    context: &EventContext<'_>,
) -> bool {
    let Some(conditions) = conditions else {
        return true;
    };

    if let Some(night) = conditions.night {
        if context.night != night {
            return false;
        }
    }
    if let Some(dangerous) = conditions.dangerous {
        if context.dangerous != dangerous {
            return false;
        }
    }
    if let Some(min_day) = conditions.min_day {
        if state.world.day < min_day {
            return false;
        }
    }
    if let Some(max_day) = conditions.max_day {
        if state.world.day > max_day {
            return false;
        }
    }
    if !conditions.locations.is_empty()
        && !conditions
            .locations
            .iter()
            .any(|location| location == context.location_name)
    {
        return false;
    }
    if let Some(prior_event_id) = conditions.prior_event_id.as_deref() {
        if !state
            .world
            .history
            .iter()
            .any(|entry| entry.event_id.as_deref() == Some(prior_event_id))
        {
            return false;
        }
    }
    if conditions.min_reputation.is_some() || conditions.max_reputation.is_some() {
        let Some(faction_name) = conditions.faction_name.as_deref() else {
            return false;
        };
        let reputation = state
            .factions
            .iter()
            .find(|faction| faction.name == faction_name)
            .map(|faction| faction.reputation)
            .unwrap_or(0);
        if let Some(min_reputation) = conditions.min_reputation {
            if reputation < min_reputation {
                return false;
            }
        }
        if let Some(max_reputation) = conditions.max_reputation {
            if reputation > max_reputation {
                return false;
            }
        }
    }
    if let Some(item_name) = conditions.required_item_name.as_deref() {
        if !state
            .character
            .inventory
            .iter()
            .any(|item| item.name == item_name)
        {
            return false;
        }
    }
    if let Some(condition_name) = conditions.required_condition_name.as_deref() {
        if !state
            .character
            .conditions
            .iter()
            .any(|condition| condition.name == condition_name)
        {
            return false;
        }
    }
    true
}

fn event_is_off_cooldown(state: &GameState, event_id: &str) -> bool {
    state
        .world
        .event_cooldowns
        .iter()
        .find(|cooldown| cooldown.event_id == event_id)
        .map(|cooldown| state.character.turn >= cooldown.ready_at_turn)
        .unwrap_or(true)
}

fn next_random_roll(state: &mut GameState) -> u64 {
    let mut rng = DeterministicRng::from_state(state.rng_state);
    let roll = rng.next_u64();
    state.rng_state = rng.state();
    roll
}

fn apply_event(state: &mut GameState, event: &EventContent, context: &EventContext<'_>) {
    let mut outcomes = Vec::new();
    for effect in &event.effects {
        apply_effect(state, effect, context, &mut outcomes);
    }

    let outcome = if outcomes.is_empty() {
        "The event leaves its mark.".to_string()
    } else {
        outcomes.join(" ")
    };
    state.world.record_event_history(
        state.character.turn,
        event.id.clone(),
        context.location_name.to_string(),
        outcome,
    );

    let cooldown_turns = event.cooldown_turns.unwrap_or(0);
    if cooldown_turns > 0 {
        let ready_at_turn = state
            .character
            .turn
            .saturating_add(cooldown_turns.saturating_add(1));
        if let Some(cooldown) = state
            .world
            .event_cooldowns
            .iter_mut()
            .find(|entry| entry.event_id == event.id)
        {
            cooldown.ready_at_turn = ready_at_turn;
        } else {
            state
                .world
                .event_cooldowns
                .push(crate::model::EventCooldown {
                    event_id: event.id.clone(),
                    ready_at_turn,
                });
        }
    }
}

fn apply_effect(
    state: &mut GameState,
    effect: &EventEffectContent,
    context: &EventContext<'_>,
    outcomes: &mut Vec<String>,
) {
    match effect {
        EventEffectContent::Message { text } => {
            let rendered = render_text(text, state, context);
            outcomes.push(rendered);
        }
        EventEffectContent::History { text } => {
            let rendered = render_text(text, state, context);
            state
                .world
                .record_history(state.character.turn, rendered.clone());
            outcomes.push(rendered);
        }
        EventEffectContent::Pause => {}
        EventEffectContent::Heal { amount } => {
            state.character.heal(*amount);
            outcomes.push(format!("Recovered {} HP.", amount));
        }
        EventEffectContent::Damage { amount } => {
            state.character.hp = (state.character.hp - amount).max(0);
            if state.character.hp == 0 {
                state.character.alive = false;
            }
            outcomes.push(format!("Suffered {} damage.", amount));
        }
        EventEffectContent::AddCondition {
            name,
            remaining,
            penalty,
            bonus,
        } => {
            let mut condition = Condition::new(name.clone(), *remaining, *penalty);
            condition.bonus = *bonus;
            crate::game::state_effects::add_or_refresh_condition(
                &mut state.character.conditions,
                condition,
            );
            outcomes.push(format!("Gained {}.", name));
        }
    }
}

fn render_text(text: &str, state: &GameState, context: &EventContext<'_>) -> String {
    text.replace("{character}", &state.character.display_name())
        .replace("{location}", context.location_name)
        .replace("{day}", &state.world.day.to_string())
}

#[cfg(test)]
#[path = "../tests/unit/events.rs"]
mod tests;
