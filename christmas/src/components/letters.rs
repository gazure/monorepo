use dioxus::prelude::*;

use crate::{model::Pool, server};

/// The letters one pool never draws, out of the flat `(pool_id, letter)` list.
fn excluded_for(pool_id: Option<i32>, all: &[(i32, char)]) -> Vec<char> {
    let Some(pool_id) = pool_id else {
        return Vec::new();
    };
    all.iter()
        .filter(|(id, _)| *id == pool_id)
        .map(|(_, letter)| *letter)
        .collect()
}

/// Per-pool letter picker. A struck-through letter is one the pool never draws.
///
/// `excluded` arrives as `(pool_id, letter)` pairs for every pool. Fetching it
/// here instead would go stale: a `use_resource` whose only input is a component
/// prop registers no reactive dependency, so it never re-runs when the pool list
/// finally loads.
#[component]
pub fn LettersSection(pools: Vec<Pool>, excluded: Vec<(i32, char)>, on_change: EventHandler<()>) -> Element {
    let first = pools.first().map(|p| p.id);
    let mut chosen = use_signal(|| None::<i32>);
    let mut error = use_signal(|| None::<String>);

    // Derived rather than synced via an effect: an effect that both reads and
    // writes the same signal re-triggers itself and never settles.
    let selected = move || chosen().or(first);

    let current = excluded_for(selected(), &excluded);

    rsx! {
        div { class: "panel",
            h3 { "Letters" }
            p { class: "muted", style: "font-size: 0.85rem; margin-top: -0.5rem",
                "Struck-through letters are never drawn. Letters this pool has already used are skipped automatically until every option is spent."
            }

            if let Some(e) = error() {
                div { class: "error-box", "{e}" }
            }

            div { class: "field-row",
                label { "Pool" }
                select {
                    value: selected().map_or_else(String::new, |v| v.to_string()),
                    onchange: move |e| chosen.set(e.value().parse().ok()),
                    for pool in pools.iter() {
                        option { key: "{pool.id}", value: "{pool.id}", "{pool.name}" }
                    }
                }
            }

            if selected().is_some() {
                div { class: "letter-grid",
                    for letter in 'A'..='Z' {
                        {
                            let off = current.contains(&letter);
                            let mut next = current.clone();
                            if off {
                                next.retain(|c| *c != letter);
                            } else {
                                next.push(letter);
                            }
                            let class = if off { "letter-toggle off" } else { "letter-toggle" };
                            let title = if off {
                                format!("{letter} is never drawn — click to allow it")
                            } else {
                                format!("{letter} may be drawn — click to rule it out")
                            };
                            rsx! {
                                button {
                                    key: "{letter}",
                                    class: "{class}",
                                    title: "{title}",
                                    onclick: move |_| {
                                        let next = next.clone();
                                        let Some(pool_id) = selected() else {
                                            return;
                                        };
                                        spawn(async move {
                                            match server::set_excluded_letters(pool_id, next).await {
                                                Ok(()) => on_change.call(()),
                                                Err(e) => error.set(Some(e.to_string())),
                                            }
                                        });
                                    },
                                    "{letter}"
                                }
                            }
                        }
                    }
                }
            } else {
                p { class: "muted", "Create a pool first." }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<(i32, char)> {
        // Grabergishimazureson (pool 2) draws from "ACDIJLMNORSTUXYZ".
        let allowed: Vec<char> = "ACDIJLMNORSTUXYZ".chars().collect();
        ('A'..='Z')
            .filter(|c| !allowed.contains(c))
            .map(|c| (2, c))
            .chain([(3, 'Q'), (3, 'Z')])
            .collect()
    }

    #[test]
    fn picks_out_only_the_selected_pool() {
        let all = sample();
        let graber = excluded_for(Some(2), &all);
        assert_eq!(graber.len(), 10);
        assert!(graber.contains(&'B'));
        assert!(!graber.contains(&'A'), "A is in the allowed set");

        assert_eq!(excluded_for(Some(3), &all), vec!['Q', 'Z']);
    }

    #[test]
    fn a_pool_with_no_exclusions_draws_the_whole_alphabet() {
        assert!(excluded_for(Some(1), &sample()).is_empty());
    }

    #[test]
    fn no_selection_excludes_nothing() {
        assert!(excluded_for(None, &sample()).is_empty());
    }
}
