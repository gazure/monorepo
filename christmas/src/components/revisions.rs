//! Going back to an earlier revision of a draw.
//!
//! Every draw, re-draw and hand adjustment is kept, so the record already holds
//! the answer when a change turns out to have been a mistake. This is the way
//! back to it: pick the revision that was right and make it the live one again.

use dioxus::prelude::*;

use crate::{
    model::{Exchange, RevisionHistory, revision_histories},
    server,
};

/// The revision history of every year that has one, with a way back.
///
/// Draws are passed in rather than fetched: a `use_resource` whose only input is
/// a component prop registers no reactive dependency, so it would never re-run
/// once the list loaded — the same trap documented in `letters.rs`.
#[component]
pub fn RevisionsSection(exchanges: Vec<Exchange>, on_change: EventHandler<()>) -> Element {
    // A year drawn once has nothing to go back to, and listing it would bury the
    // years that do.
    let histories: Vec<RevisionHistory> = revision_histories(exchanges)
        .into_iter()
        .filter(|h| h.revisions.len() > 1)
        .collect();

    // Which row is in flight, so only its button says so.
    let mut restoring = use_signal(|| None::<i32>);
    let mut error = use_signal(|| None::<String>);
    let mut restored = use_signal(|| None::<Exchange>);

    let mut restore = move |id: i32| {
        restoring.set(Some(id));
        error.set(None);
        restored.set(None);
        spawn(async move {
            match server::restore_revision(id).await {
                Ok(exchange) => {
                    restored.set(Some(exchange));
                    on_change.call(());
                }
                Err(e) => error.set(Some(e.to_string())),
            }
            restoring.set(None);
        });
    };

    rsx! {
        div { class: "panel",
            h3 { "Revision history" }
            p { class: "muted", style: "font-size: 0.85rem; margin-top: -0.5rem",
                "Restoring puts an earlier revision back in charge by recording it again as the newest one. Nothing is overwritten, so a restore can itself be undone."
            }

            if let Some(e) = error() {
                div { class: "error-box", "{e}" }
            }

            if let Some(exchange) = restored() {
                div { class: "notice",
                    "{exchange.pool_name} {exchange.year} is back to those pairings, saved as revision {exchange.revision}."
                }
            }

            if histories.is_empty() {
                p { class: "muted", "No year has been drawn more than once yet." }
            } else {
                for history in histories.iter() {
                    div { key: "{history.pool_id}-{history.year}", style: "margin-top: 1.25rem",
                        div { class: "section-head",
                            h2 { style: "font-size: 1.05rem", "{history.pool_name} · {history.year}" }
                            span { class: "count", "{history.revisions.len()} revisions" }
                        }
                        div { class: "table-scroll",
                            table { class: "data-table",
                                thead {
                                    tr {
                                        th { "Revision" }
                                        th { "What it was" }
                                        th { "Letter" }
                                        th { "People" }
                                        th { "Rings" }
                                        th { "" }
                                    }
                                }
                                tbody {
                                    for (position , exchange) in history.revisions.iter().enumerate() {
                                        {
                                            // Sorted highest revision first, so the head is live.
                                            let is_live = position == 0;
                                            let id = exchange.id;
                                            rsx! {
                                                tr { key: "{id}",
                                                    td { class: "mono", "{exchange.revision}" }
                                                    td { "{provenance(exchange)}" }
                                                    td { {exchange.letter.map_or_else(|| "—".to_string(), |c| c.to_string())} }
                                                    td { class: "mono", "{exchange.participants.len()}" }
                                                    td { class: "mono", "{exchange.cycles().len()}" }
                                                    td {
                                                        if is_live {
                                                            span { class: "badge current", "current" }
                                                        } else {
                                                            button {
                                                                class: "ghost",
                                                                style: "padding: 0.25rem 0.6rem; font-size: 0.85rem",
                                                                disabled: restoring().is_some(),
                                                                onclick: move |_| restore(id),
                                                                if restoring() == Some(id) {
                                                                    "Restoring…"
                                                                } else {
                                                                    "Restore"
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Where a revision came from, in a few words.
///
/// A hand edit already describes itself. Everything else is told apart by
/// whether the draw was recorded with the rules it ran under: the solver always
/// writes them, and a backfilled year deliberately does not, because nobody
/// knows what rules a paper draw from 2014 followed.
fn provenance(exchange: &Exchange) -> String {
    exchange.adjustment_note.clone().unwrap_or_else(|| {
        if exchange.config.is_some() {
            "Drawn".to_string()
        } else {
            "Entered by hand".to_string()
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::DrawConfig;

    fn exchange(revision: i32) -> Exchange {
        Exchange {
            id: revision,
            pool_id: 1,
            pool_name: "Pets".into(),
            pool_slug: "pets".into(),
            year: 2026,
            revision,
            letter: None,
            participants: vec![],
            exclusions: vec![],
            pairings: vec![],
            config: None,
            seed: None,
            adjusted_from: None,
            adjustment_note: None,
        }
    }

    #[test]
    fn a_solver_draw_says_so() {
        let drawn = Exchange {
            config: Some(DrawConfig::default()),
            ..exchange(1)
        };
        assert_eq!(provenance(&drawn), "Drawn");
    }

    /// A backfill carries no config precisely because its rules are unknown.
    #[test]
    fn a_backfilled_year_is_not_passed_off_as_a_draw() {
        assert_eq!(provenance(&exchange(1)), "Entered by hand");
    }

    /// Including a restore, whose note is what makes the trail readable.
    #[test]
    fn an_edited_revision_speaks_for_itself() {
        let adjusted = Exchange {
            config: Some(DrawConfig::default()),
            adjusted_from: Some(1),
            adjustment_note: Some("Swapped Alec and Noel".into()),
            ..exchange(2)
        };
        assert_eq!(provenance(&adjusted), "Swapped Alec and Noel");

        let restored = Exchange {
            adjusted_from: Some(1),
            adjustment_note: Some("Restored revision 1".into()),
            ..exchange(3)
        };
        assert_eq!(provenance(&restored), "Restored revision 1");
    }
}
