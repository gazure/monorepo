//! Entering a draw that happened before the app existed.
//!
//! The reason to bother: "no repeat receivers" can only avoid what it can see,
//! so a pool with no recorded history has nothing to avoid. Typing last year's
//! receivers in once gives this year's draw something to work against.

use std::collections::BTreeMap;

use dioxus::prelude::*;

use crate::{
    model::{Exchange, Pairing, Participant, Pool},
    server,
};

/// Hand-entered history for one pool and year.
///
/// Everything is passed in rather than fetched here: a `use_resource` whose only
/// input is a component prop registers no reactive dependency, so it would never
/// re-run once the lists loaded — the same trap documented in `letters.rs`.
#[component]
pub fn BackfillSection(
    pools: Vec<Pool>,
    participants: Vec<Participant>,
    memberships: Vec<(i32, i32)>,
    exchanges: Vec<Exchange>,
    year: i32,
    on_change: EventHandler<()>,
) -> Element {
    let first = pools.first().map(|p| p.id);
    let mut chosen_pool = use_signal(|| None::<i32>);
    // Backfilling is almost always about the year just gone.
    let mut back_year = use_signal(|| year - 1);
    let mut letter = use_signal(|| None::<char>);

    // Giver name → receiver name. Sorted, so the summary reads in a stable order.
    let mut picks = use_signal(BTreeMap::<String, String>::new);

    let mut saving = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut saved = use_signal(|| None::<Exchange>);

    // Derived, not synced through an effect — see the note in `letters.rs`.
    let pool_id = move || chosen_pool().or(first);

    let members: Vec<String> = participants
        .iter()
        .filter(|p| {
            memberships
                .iter()
                .any(|(pool, participant)| Some(*pool) == pool_id() && *participant == p.id)
        })
        .map(|p| p.name.clone())
        .collect();

    // What is already on the record for this pool and year, if anything. Newest
    // revision first, so this is the live one.
    let existing = exchanges
        .iter()
        .find(|e| Some(e.pool_id) == pool_id() && e.year == back_year())
        .cloned();

    // Two givers cannot share a receiver. Flagged as it is typed, because
    // finding out on save which of fourteen selects is wrong is miserable.
    let clashes: Vec<String> = {
        let chosen = picks.read();
        let mut counts = BTreeMap::<&str, usize>::new();
        for receiver in chosen.values() {
            *counts.entry(receiver.as_str()).or_default() += 1;
        }
        counts
            .into_iter()
            .filter(|(_, n)| *n > 1)
            .map(|(name, _)| name.to_string())
            .collect()
    };
    let filled = picks.read().len();
    let ready = filled > 0 && clashes.is_empty();

    let mut load_existing = {
        let existing = existing.clone();
        move |()| {
            let Some(draw) = existing.as_ref() else { return };
            let mut loaded = BTreeMap::new();
            for pair in &draw.pairings {
                loaded.insert(pair.giver.clone(), pair.receiver.clone());
            }
            picks.set(loaded);
            letter.set(draw.letter);
            saved.set(None);
            error.set(None);
        }
    };

    let save = move |_| {
        let Some(id) = pool_id() else {
            error.set(Some("Pick a pool".to_string()));
            return;
        };
        let pairings: Vec<Pairing> = picks
            .read()
            .iter()
            .map(|(giver, receiver)| Pairing {
                giver: giver.clone(),
                receiver: receiver.clone(),
            })
            .collect();
        let y = back_year();
        let l = letter();

        saving.set(true);
        error.set(None);
        spawn(async move {
            match server::record_past_draw(id, y, pairings, l).await {
                Ok(exchange) => {
                    saved.set(Some(exchange));
                    error.set(None);
                    picks.set(BTreeMap::new());
                    letter.set(None);
                    on_change.call(());
                }
                Err(e) => {
                    saved.set(None);
                    error.set(Some(e.to_string()));
                }
            }
            saving.set(false);
        });
    };

    rsx! {
        div { class: "panel",
            h3 { "Backfill a past year" }
            p { class: "muted", style: "font-size: 0.85rem; margin-top: -0.5rem",
                "Type in who gave to whom in an earlier year. Once it's on the record, \"no repeat receivers\" can see it and its letter won't be drawn again. Leave anyone you can't remember blank."
            }

            if let Some(e) = error() {
                div { class: "error-box", "{e}" }
            }

            div { class: "field-row",
                label { "Pool" }
                select {
                    value: pool_id().map_or_else(String::new, |v| v.to_string()),
                    onchange: move |e| {
                        chosen_pool.set(e.value().parse().ok());
                        // The old pool's names mean nothing in the new one.
                        picks.set(BTreeMap::new());
                        saved.set(None);
                    },
                    for pool in pools.iter() {
                        option { key: "{pool.id}", value: "{pool.id}", "{pool.name}" }
                    }
                }
                label { "Year" }
                input {
                    r#type: "number",
                    style: "width: 6.5rem",
                    value: "{back_year}",
                    oninput: move |e| {
                        if let Ok(v) = e.value().parse() {
                            back_year.set(v);
                            saved.set(None);
                        }
                    },
                }
                label { "Letter" }
                select {
                    style: "width: 5rem",
                    value: letter().map_or_else(String::new, |c| c.to_string()),
                    onchange: move |e| letter.set(e.value().chars().next()),
                    option { value: "", "—" }
                    for c in 'A'..='Z' {
                        option { key: "{c}", value: "{c}", "{c}" }
                    }
                }
            }

            if let Some(draw) = existing.as_ref() {
                div { class: "notice",
                    "{back_year} is already recorded for this pool "
                    if draw.pairings.is_empty() {
                        "with no pairings"
                    } else {
                        "with {draw.pairings.len()} pairings"
                    }
                    ". Saving records a new revision rather than replacing it. "
                    button {
                        class: "linklike",
                        style: "text-transform: none; letter-spacing: 0",
                        onclick: move |_| load_existing(()),
                        "Load it to edit"
                    }
                }
            }

            if members.is_empty() {
                p { class: "muted", "This pool has no members yet — add people before backfilling." }
            } else {
                div { class: "backfill-grid",
                    for giver in members.iter() {
                        {
                            let giver = giver.clone();
                            let chosen = picks.read().get(&giver).cloned().unwrap_or_default();
                            let clashing = !chosen.is_empty() && clashes.contains(&chosen);
                            let others = members.clone();
                            let key = giver.clone();
                            rsx! {
                                div { key: "{key}", class: "backfill-row",
                                    span { class: "giver", "{giver}" }
                                    span { class: "arrow", "→" }
                                    select {
                                        class: if clashing { "clashing" } else { "" },
                                        value: "{chosen}",
                                        onchange: move |e| {
                                            let picked = e.value();
                                            if picked.is_empty() {
                                                picks.write().remove(&giver);
                                            } else {
                                                picks.write().insert(giver.clone(), picked);
                                            }
                                            saved.set(None);
                                        },
                                        option { value: "", "—" }
                                        for other in others.iter().filter(|o| **o != key) {
                                            option { key: "{other}", value: "{other}", "{other}" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if !clashes.is_empty() {
                    div { class: "error-box", style: "margin-top: 1rem",
                        "Two people are down as giving to "
                        {clashes.join(", ")}
                        ". Everyone receives from exactly one person."
                    }
                }

                div { class: "field-row", style: "margin-top: 1rem",
                    button { disabled: saving() || !ready, onclick: save,
                        if saving() { "Saving…" } else { "Record {back_year}" }
                    }
                    span { class: "muted", style: "font-size: 0.85rem",
                        if filled == 0 {
                            "Nothing filled in yet."
                        } else if filled == members.len() {
                            "All {filled} filled in."
                        } else {
                            "{filled} of {members.len()} filled in — the rest will be left out."
                        }
                    }
                }
            }

            if let Some(exchange) = saved() {
                div { class: "notice", style: "margin-top: 1rem",
                    "Recorded {exchange.pairings.len()} pairings for {exchange.year}"
                    if let Some(letter) = exchange.letter {
                        ", letter {letter}"
                    }
                    " as revision {exchange.revision}."
                }
            }
        }
    }
}
