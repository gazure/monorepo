use dioxus::prelude::*;

use crate::{
    app::Route,
    model::{CycleMode, DEFAULT_MIN_CYCLE_LEN, DrawConfig, Exchange, Pool},
    server,
};

/// `exchanges` is passed in rather than fetched here: a `use_resource` whose
/// only input is a component prop registers no reactive dependency, so it would
/// never re-run once the pool list loaded and would report "no history" forever.
#[component]
pub fn DrawSection(pools: Vec<Pool>, exchanges: Vec<Exchange>, year: i32, on_change: EventHandler<()>) -> Element {
    let first = pools.first().map(|p| p.id);
    let mut chosen_pool = use_signal(|| None::<i32>);
    let mut draw_year = use_signal(|| year);

    // Derived, not synced via an effect — see the note in `letters.rs`.
    let pool_id = move || chosen_pool().or(first);

    // Defaults: one grand ring, spouses kept apart, no repeat of last year.
    let mut grand = use_signal(|| true);
    let mut min_len = use_signal(|| DEFAULT_MIN_CYCLE_LEN);
    let mut exclude_spouses = use_signal(|| true);
    let mut avoid_repeats = use_signal(|| true);
    let mut lookback = use_signal(|| 1u32);
    let mut include_letter = use_signal(|| true);

    let mut running = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut result = use_signal(|| None::<Exchange>);

    // Used to tell the user when "no repeat" has nothing to bite on yet.
    let prior_years: usize = {
        let mut years: Vec<i32> = exchanges
            .iter()
            .filter(|e| Some(e.pool_id) == pool_id() && e.year < draw_year())
            .map(|e| e.year)
            .collect();
        years.sort_unstable();
        years.dedup();
        years.len()
    };

    let run = move |_| {
        let Some(id) = pool_id() else {
            error.set(Some("Pick a pool".to_string()));
            return;
        };
        let config = DrawConfig {
            cycle_mode: if grand() {
                CycleMode::Grand
            } else {
                CycleMode::Multiple { min_len: min_len() }
            },
            exclude_spouses: exclude_spouses(),
            avoid_repeat_years: if avoid_repeats() { Some(lookback()) } else { None },
            // Zero asks the server for a fresh seed.
            seed: 0,
        };
        let y = draw_year();
        let letter = include_letter();

        running.set(true);
        error.set(None);
        spawn(async move {
            match server::run_draw(id, y, config, letter).await {
                Ok(exchange) => {
                    result.set(Some(exchange));
                    error.set(None);
                    on_change.call(());
                }
                Err(e) => {
                    result.set(None);
                    error.set(Some(e.to_string()));
                }
            }
            running.set(false);
        });
    };

    rsx! {
        div { class: "panel",
            h3 { "Run a draw" }
            p { class: "muted", style: "font-size: 0.85rem; margin-top: -0.5rem",
                "Drawing again never overwrites an earlier result — it records a new revision."
            }

            if let Some(e) = error() {
                div { class: "error-box", "{e}" }
            }

            div { class: "field-row",
                label { "Pool" }
                select {
                    value: pool_id().map_or_else(String::new, |v| v.to_string()),
                    onchange: move |e| chosen_pool.set(e.value().parse().ok()),
                    for pool in pools.iter() {
                        option { key: "{pool.id}", value: "{pool.id}", "{pool.name}" }
                    }
                }
                label { "Year" }
                input {
                    r#type: "number",
                    style: "width: 6.5rem",
                    value: "{draw_year}",
                    oninput: move |e| {
                        if let Ok(v) = e.value().parse() {
                            draw_year.set(v);
                        }
                    },
                }
            }

            div { class: "toggle-row",
                input {
                    r#type: "checkbox",
                    id: "grand",
                    checked: grand(),
                    onchange: move |e| grand.set(e.checked()),
                }
                label { r#for: "grand",
                    "One grand ring"
                    span { class: "hint", "Everyone in a single chain. Turn off for several smaller rings." }
                }
            }

            if !grand() {
                div { class: "field-row", style: "margin-left: 1.8rem",
                    label { "Smallest ring" }
                    input {
                        r#type: "number",
                        style: "width: 5rem",
                        min: "3",
                        value: "{min_len}",
                        oninput: move |e| {
                            if let Ok(v) = e.value().parse::<usize>() {
                                min_len.set(v.max(3));
                            }
                        },
                    }
                    span { class: "muted", style: "font-size: 0.8rem",
                        "Three is the floor — a ring of two is just swapping gifts."
                    }
                }
            }

            div { class: "toggle-row",
                input {
                    r#type: "checkbox",
                    id: "spouses",
                    checked: exclude_spouses(),
                    onchange: move |e| exclude_spouses.set(e.checked()),
                }
                label { r#for: "spouses",
                    "Keep spouses apart"
                    span { class: "hint", "Applies to everyone linked as a spouse or housemate." }
                }
            }

            div { class: "toggle-row",
                input {
                    r#type: "checkbox",
                    id: "repeats",
                    checked: avoid_repeats(),
                    onchange: move |e| avoid_repeats.set(e.checked()),
                }
                label { r#for: "repeats",
                    "No repeat receivers"
                    span { class: "hint",
                        if prior_years == 0 {
                            "On, but there's no earlier draw for this pool yet — it starts applying next year."
                        } else if lookback() == 1 {
                            "Nobody gives to the person they gave to last year."
                        } else {
                            "Nobody gives to anyone they've given to in the last {lookback} years."
                        }
                    }
                }
            }

            if avoid_repeats() && prior_years > 0 {
                div { class: "field-row", style: "margin-left: 1.8rem",
                    label { "Years to look back" }
                    input {
                        r#type: "number",
                        style: "width: 5rem",
                        min: "1",
                        value: "{lookback}",
                        oninput: move |e| {
                            if let Ok(v) = e.value().parse::<u32>() {
                                lookback.set(v.max(1));
                            }
                        },
                    }
                }
            }

            div { class: "toggle-row",
                input {
                    r#type: "checkbox",
                    id: "letter",
                    checked: include_letter(),
                    onchange: move |e| include_letter.set(e.checked()),
                }
                label { r#for: "letter",
                    "Pick a letter"
                    span { class: "hint", "Skips letters this pool has already used." }
                }
            }

            button { disabled: running(), onclick: run,
                if running() { "Drawing…" } else { "Draw" }
            }

            if let Some(exchange) = result() {
                div { class: "notice", style: "margin-top: 1rem",
                    "Drew {exchange.pairings.len()} pairings across {exchange.cycles().len()} ring(s) for {exchange.year}"
                    if let Some(letter) = exchange.letter {
                        ", letter {letter}"
                    }
                    ". Recorded as revision {exchange.revision}."
                }
                Link {
                    class: "reveal-cta",
                    to: Route::Reveal { id: exchange.id },
                    "Watch the reveal →"
                }
            }
        }
    }
}
