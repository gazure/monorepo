use dioxus::prelude::*;

use super::current_year;
use crate::{
    components::{DrawSection, LettersSection, ParticipantsSection, PoolsSection, RelationshipsSection},
    server,
};

/// Resources carry different payload types, so this has to be generic — a
/// closure would fix itself to whichever type it saw first.
fn err_of<T>(resource: Option<&Result<T, ServerFnError>>) -> Option<String> {
    match resource {
        Some(Err(e)) => Some(e.to_string()),
        _ => None,
    }
}

#[component]
pub fn Admin() -> Element {
    let mut pools = use_resource(server::list_pools);
    let mut participants = use_resource(server::list_participants);
    let mut relationships = use_resource(server::list_relationships);
    let mut memberships = use_resource(server::list_memberships);
    let mut exchanges = use_resource(move || server::list_exchanges(None));
    let mut letters = use_resource(server::list_all_excluded_letters);

    let reload_all = move |()| {
        pools.restart();
        participants.restart();
        relationships.restart();
        memberships.restart();
        exchanges.restart();
        letters.restart();
    };

    let pool_list = match &*pools.read() {
        Some(Ok(list)) => list.clone(),
        _ => Vec::new(),
    };
    let participant_list = match &*participants.read() {
        Some(Ok(list)) => list.clone(),
        _ => Vec::new(),
    };
    let relationship_list = match &*relationships.read() {
        Some(Ok(list)) => list.clone(),
        _ => Vec::new(),
    };
    let membership_list = match &*memberships.read() {
        Some(Ok(list)) => list.clone(),
        _ => Vec::new(),
    };
    let exchange_list = match &*exchanges.read() {
        Some(Ok(list)) => list.clone(),
        _ => Vec::new(),
    };
    let letter_list: Vec<(i32, char)> = match &*letters.read() {
        Some(Ok(list)) => list
            .iter()
            .filter_map(|(pool_id, letter)| letter.chars().next().map(|c| (*pool_id, c)))
            .collect(),
        _ => Vec::new(),
    };

    let load_error = err_of(pools.read().as_ref())
        .or_else(|| err_of(participants.read().as_ref()))
        .or_else(|| err_of(relationships.read().as_ref()));

    rsx! {
        header { class: "hero",
            div { class: "hero-copy",
                p { class: "eyebrow", "Behind the curtain" }
                h1 { "Manage" }
                p { "Pools, people, and the rules each draw runs under." }
            }
        }

        if let Some(e) = load_error {
            div { class: "error-box", "Couldn't load: {e}" }
        }

        DrawSection {
            pools: pool_list.clone(),
            exchanges: exchange_list,
            year: current_year(),
            on_change: reload_all,
        }

        PoolsSection { pools: pool_list.clone(), on_change: reload_all }

        ParticipantsSection {
            participants: participant_list.clone(),
            pools: pool_list.clone(),
            memberships: membership_list,
            on_change: reload_all,
        }

        RelationshipsSection {
            participants: participant_list,
            relationships: relationship_list,
            on_change: reload_all,
        }

        LettersSection { pools: pool_list, excluded: letter_list, on_change: reload_all }
    }
}
