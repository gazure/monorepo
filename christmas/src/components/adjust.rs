//! Trading two people's places in a ring, after the draw has run.
//!
//! The draw is a good solver but a bad judge of circumstance — somebody moved,
//! somebody is unwell, somebody already bought the wrong present. Re-running the
//! whole thing to fix one pairing throws away thirteen good ones, so this moves
//! only the two that need moving.

use dioxus::prelude::*;

use crate::{
    model::{Exchange, Pool, SwapPreview, only_current_revisions},
    server,
};

/// A hand adjustment to one recorded draw.
///
/// Draws are passed in rather than fetched: a `use_resource` whose only input is
/// a component prop registers no reactive dependency, so it would never re-run
/// once the list loaded — the same trap documented in `letters.rs`.
#[component]
pub fn AdjustSection(pools: Vec<Pool>, exchanges: Vec<Exchange>, on_change: EventHandler<()>) -> Element {
    // Only the live revision of each year can be adjusted, and only if it has
    // pairings to adjust — the server enforces both, but offering the choice and
    // then refusing it would be a poor way to say so.
    let adjustable: Vec<Exchange> = only_current_revisions(exchanges)
        .into_iter()
        .filter(|e| !e.pairings.is_empty())
        .collect();

    let first = adjustable.first().map(|e| e.id);
    let mut chosen = use_signal(|| None::<i32>);
    let mut a = use_signal(String::new);
    let mut b = use_signal(String::new);
    let mut preview = use_signal(|| None::<SwapPreview>);
    let mut despite = use_signal(|| false);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut saved = use_signal(|| None::<Exchange>);

    // Derived, not synced through an effect — see the note in `letters.rs`.
    let draw_id = move || chosen().or(first);
    let draw = adjustable.iter().find(|e| Some(e.id) == draw_id()).cloned();

    // Swapping across rings would splice them into one, so the second dropdown
    // offers only the first person's own ring. In `Grand` mode that is everyone,
    // which is the common case; in `Multiple` mode it keeps the rings intact.
    let rings = draw.as_ref().map(Exchange::cycles).unwrap_or_default();
    let ring_of_a: Vec<String> = rings
        .iter()
        .find(|ring| ring.iter().any(|name| *name == a()))
        .cloned()
        .unwrap_or_default();
    let partners: Vec<String> = ring_of_a.iter().filter(|name| **name != a()).cloned().collect();

    let everyone: Vec<String> = draw.as_ref().map(|d| d.participants.clone()).unwrap_or_default();
    let ready = !a().is_empty() && !b().is_empty() && a() != b();

    let mut reset = move || {
        preview.set(None);
        despite.set(false);
        error.set(None);
        saved.set(None);
    };

    let do_preview = move |_| {
        let (Some(id), first, second) = (draw_id(), a(), b()) else {
            return;
        };
        busy.set(true);
        error.set(None);
        saved.set(None);
        spawn(async move {
            match server::preview_swap(id, first, second).await {
                Ok(p) => {
                    // A fresh preview invalidates any previous override.
                    despite.set(false);
                    preview.set(Some(p));
                }
                Err(e) => {
                    preview.set(None);
                    error.set(Some(e.to_string()));
                }
            }
            busy.set(false);
        });
    };

    let apply = move |_| {
        let (Some(id), first, second) = (draw_id(), a(), b()) else {
            return;
        };
        let confirmed = despite();
        busy.set(true);
        error.set(None);
        spawn(async move {
            match server::apply_swap(id, first, second, confirmed).await {
                Ok(exchange) => {
                    saved.set(Some(exchange));
                    preview.set(None);
                    despite.set(false);
                    a.set(String::new());
                    b.set(String::new());
                    error.set(None);
                    on_change.call(());
                }
                Err(e) => error.set(Some(e.to_string())),
            }
            busy.set(false);
        });
    };

    let pool_label = |exchange: &Exchange| {
        let name = pools
            .iter()
            .find(|p| p.id == exchange.pool_id)
            .map_or(exchange.pool_name.clone(), |p| p.name.clone());
        format!("{name} · {}", exchange.year)
    };

    rsx! {
        div { class: "panel",
            h3 { "Adjust a draw by hand" }
            p { class: "muted", style: "font-size: 0.85rem; margin-top: -0.5rem",
                "Swap two people's places in the ring when the draw is right but life isn't. Everyone else keeps who they've got. The change is saved as a new revision, so the original is still on the record."
            }

            if let Some(e) = error() {
                div { class: "error-box", "{e}" }
            }

            if adjustable.is_empty() {
                p { class: "muted", "Nothing to adjust yet — run a draw first." }
            } else {
                div { class: "field-row",
                    label { "Draw" }
                    select {
                        value: draw_id().map_or_else(String::new, |v| v.to_string()),
                        onchange: move |e| {
                            chosen.set(e.value().parse().ok());
                            // Names from the old draw mean nothing in the new one.
                            a.set(String::new());
                            b.set(String::new());
                            reset();
                        },
                        for exchange in adjustable.iter() {
                            option { key: "{exchange.id}", value: "{exchange.id}", "{pool_label(exchange)}" }
                        }
                    }
                }

                div { class: "field-row",
                    label { "Swap" }
                    select {
                        value: "{a}",
                        onchange: move |e| {
                            a.set(e.value());
                            // The new person may be in a different ring.
                            b.set(String::new());
                            reset();
                        },
                        option { value: "", "—" }
                        for name in everyone.iter() {
                            option { key: "{name}", value: "{name}", "{name}" }
                        }
                    }
                    label { "with" }
                    select {
                        value: "{b}",
                        disabled: a().is_empty(),
                        onchange: move |e| {
                            b.set(e.value());
                            reset();
                        },
                        option { value: "", "—" }
                        for name in partners.iter() {
                            option { key: "{name}", value: "{name}", "{name}" }
                        }
                    }
                    button { disabled: busy() || !ready, onclick: do_preview,
                        if busy() { "Working…" } else { "Preview" }
                    }
                }

                if rings.len() > 1 && !a().is_empty() {
                    div { class: "notice",
                        "This draw has {rings.len()} rings. Only the {partners.len()} others in {a}'s ring are offered — swapping across rings would join them into one."
                    }
                }

                if let Some(p) = preview() {
                    SwapPreviewPanel {
                        preview: p,
                        despite: despite(),
                        busy: busy(),
                        on_despite: move |v| despite.set(v),
                        on_apply: apply,
                    }
                }
            }

            if let Some(exchange) = saved() {
                div { class: "notice", style: "margin-top: 1rem",
                    if let Some(note) = exchange.adjustment_note.as_ref() {
                        "{note}. "
                    }
                    "Saved as revision {exchange.revision} of {exchange.year}."
                }
            }
        }
    }
}

/// What the swap would do, and the confirmation it needs.
///
/// Split out because a swap moves up to four pairings, not the two its name
/// suggests, and showing that before it is saved is the whole point of having a
/// preview step.
#[component]
fn SwapPreviewPanel(
    preview: SwapPreview,
    despite: bool,
    busy: bool,
    on_despite: EventHandler<bool>,
    on_apply: EventHandler<MouseEvent>,
) -> Element {
    let blocked = !preview.violations.is_empty() && !despite;

    rsx! {
        div { style: "margin-top: 1rem",
            div { class: "section-head",
                h2 { style: "font-size: 1.1rem", "Swapping {preview.a} and {preview.b}" }
                span { class: "count", "{preview.changes.len()} pairings move" }
            }

            div { class: "table-scroll",
                table { class: "data-table",
                    thead {
                        tr {
                            th { "Giver" }
                            th { "Was giving to" }
                            th { "Would give to" }
                        }
                    }
                    tbody {
                        for change in preview.changes.iter() {
                            tr { key: "{change.giver}",
                                td { class: "giver", "{change.giver}" }
                                td { class: "muted", "{change.was}" }
                                td { class: "receiver", "{change.now}" }
                            }
                        }
                    }
                }
            }

            if preview.changes.is_empty() {
                div { class: "notice", "That swap changes nothing." }
            }

            if !preview.violations.is_empty() {
                div { class: "error-box", style: "margin-top: 1rem",
                    strong { "This breaks the rules the draw ran under:" }
                    ul { style: "margin: 0.5rem 0 0; padding-left: 1.1rem",
                        for violation in preview.violations.iter() {
                            li { key: "{violation.giver}-{violation.receiver}",
                                "{violation.giver} would give to {violation.receiver} — {violation.reason}"
                            }
                        }
                    }
                }
                label { class: "muted", style: "display: block; font-size: 0.85rem; margin-bottom: 1rem",
                    input {
                        r#type: "checkbox",
                        checked: despite,
                        style: "margin-right: 0.4rem",
                        onchange: move |e| on_despite.call(e.checked()),
                    }
                    "I know — do it anyway."
                }
            }

            button {
                disabled: busy || blocked || preview.changes.is_empty(),
                onclick: move |e| on_apply.call(e),
                if busy { "Saving…" } else { "Apply the swap" }
            }
        }
    }
}
