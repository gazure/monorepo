use dioxus::prelude::*;

use crate::{
    app::Route,
    auth::Role,
    components::CycleBoard,
    model::{Exchange, PoolDetail},
    pages::Ceremony,
    server, storage,
};

#[component]
pub fn PoolPage(slug: String) -> Element {
    let detail = use_resource({
        let slug = slug.clone();
        move || server::pool_detail(slug.clone())
    });

    rsx! {
        match &*detail.read() {
            Some(Ok(detail)) => rsx! { PoolBody { detail: detail.clone() } },
            Some(Err(e)) => rsx! { div { class: "error-box", "{e}" } },
            None => rsx! { div { class: "loading", "Loading pool…" } },
        }
    }
}

#[component]
fn PoolBody(detail: PoolDetail) -> Element {
    // Exchanges arrive newest-first, so the head is the current draw.
    let current = detail.exchanges.first().cloned();
    let draw_id = current.as_ref().map(|c| c.id);

    // `None` until the browser has been consulted. Local storage is unavailable
    // during server rendering, so the first paint must not reveal anything —
    // otherwise a returning visitor flashes the results before the check runs,
    // and a new one has the whole draw spoiled.
    // `None` until the browser has been consulted. Local storage is unavailable
    // during server rendering, so the first paint must not reveal anything —
    // otherwise a returning visitor flashes the gate, and a new one has the
    // whole draw spoiled.
    let mut watched = use_signal(|| None::<bool>);
    let mut ceremony_open = use_signal(|| false);

    use_effect(move || {
        let Some(id) = draw_id else {
            watched.set(Some(true));
            return;
        };
        let already = storage::has_watched(id);
        watched.set(Some(already));
        // First time this browser has seen this draw: play it unprompted.
        if !already {
            ceremony_open.set(true);
        }
    });

    // Watching it through and skipping out both count as having had the chance,
    // so either way the results come out and it stops playing uninvited.
    let mut done_with_it = move || {
        if let Some(id) = draw_id {
            storage::mark_watched(id);
        }
        watched.set(Some(true));
        ceremony_open.set(false);
    };

    let is_revealed = watched() == Some(true);

    // A "revision 5" badge would tell a viewer there are earlier draws they
    // cannot see, which is exactly what hiding them is meant to avoid.
    let role = use_resource(server::my_role);
    let is_manager = matches!(&*role.read(), Some(Ok(Role::Manager)));

    // One row per past year. Managers can see superseded revisions elsewhere,
    // but listing them here labelled "Earlier years" would just be wrong.
    let earlier_years: Vec<Exchange> = {
        let current_year = current.as_ref().map(|c| c.year);
        let mut seen = std::collections::HashSet::new();
        detail
            .exchanges
            .iter()
            .filter(|e| Some(e.year) != current_year)
            .filter(|e| seen.insert(e.year))
            .cloned()
            .collect()
    };

    rsx! {
        if ceremony_open() {
            if let Some(draw) = current.clone() {
                Ceremony {
                    exchange: draw,
                    autoplay: true,
                    on_watched: move |()| done_with_it(),
                    on_close: Some(EventHandler::new(move |()| done_with_it())),
                }
            }
        }

        header { class: "hero",
            div { class: "hero-copy",
                p { class: "eyebrow",
                    match current.as_ref() {
                        Some(c) => rsx! { "Christmas {c.year}" },
                        None => rsx! { "Not yet drawn" },
                    }
                }
                h1 { "{detail.pool.name}" }
                if let Some(desc) = detail.pool.description.as_ref() {
                    p { "{desc}" }
                }
            }
            if is_revealed {
                if let Some(letter) = current.as_ref().and_then(|c| c.letter) {
                    div { class: "letter-mark",
                        div { class: "letter-glyph", "{letter}" }
                        div { class: "letter-caption", "gifts start here" }
                    }
                }
            }
        }

        match current.as_ref() {
            // Nothing about the draw is shown until it has been watched or
            // deliberately skipped.
            Some(draw) if !draw.pairings.is_empty() && !is_revealed => rsx! {
                div { class: "reveal-gate",
                    strong { "Not opened yet" }
                    p {
                        "The {draw.year} draw is in. Watch it unfold, or skip straight to the names."
                    }
                    div { class: "gate-actions",
                        button { onclick: move |_| ceremony_open.set(true), "Watch the draw" }
                        button { class: "ghost", onclick: move |_| done_with_it(), "Skip →" }
                    }
                }
            },
            Some(draw) if !draw.pairings.is_empty() => rsx! {
                section { class: "section",
                    div { class: "section-head",
                        h2 { "The ring" }
                        if is_manager && draw.revision > 1 {
                            span { class: "badge", "revision {draw.revision}" }
                        }
                        button {
                            class: "linklike",
                            onclick: move |_| ceremony_open.set(true),
                            "Replay the reveal"
                        }
                    }
                    CycleBoard { cycles: draw.cycles(), letter: draw.letter }
                }

                section { class: "section",
                    div { class: "section-head",
                        h2 { "Every pairing" }
                        span { class: "count", "{draw.pairings.len()}" }
                    }
                    ul { class: "pairings",
                        for pair in draw.pairings.iter() {
                            li { key: "{pair.giver}",
                                span { class: "giver", "{pair.giver}" }
                                span { class: "arrow", "→" }
                                span { class: "receiver", "{pair.receiver}" }
                            }
                        }
                    }
                }

                DrawSettings { draw: draw.clone() }
            },
            _ => rsx! {
                div { class: "empty-cta",
                    strong { "This pool hasn't drawn yet" }
                    "Head to Manage to run the draw."
                }
            },
        }

        section { class: "section",
            div { class: "section-head",
                h2 { "Members" }
                span { class: "count", "{detail.members.len()}" }
            }
            ul { class: "pairings",
                for member in detail.members.iter() {
                    li { key: "{member.id}", span { class: "giver", "{member.name}" } }
                }
            }
        }

        if !earlier_years.is_empty() {
            section { class: "section",
                div { class: "section-head", h2 { "Earlier years" } }
                div { class: "table-scroll",
                    table { class: "data-table",
                        thead {
                            tr {
                                th { "Year" }
                                th { "Letter" }
                                th { "People" }
                                th { "Rings" }
                                th { "" }
                            }
                        }
                        tbody {
                            for ex in earlier_years.iter() {
                                tr { key: "{ex.id}",
                                    td { class: "mono", "{ex.year}" }
                                    td { {ex.letter.map_or_else(|| "—".to_string(), |c| c.to_string())} }
                                    td { class: "mono", "{ex.participants.len()}" }
                                    td { class: "mono", "{ex.cycles().len()}" }
                                    td {
                                        Link { to: Route::YearPage { year: ex.year }, "view" }
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

/// Shows the rules the draw ran under, so a result can be explained later.
#[component]
fn DrawSettings(draw: Exchange) -> Element {
    let Some(config) = draw.config.as_ref() else {
        return rsx! {};
    };

    let mode = match config.cycle_mode {
        crate::model::CycleMode::Grand => "one grand ring".to_string(),
        crate::model::CycleMode::Multiple { min_len } => format!("multiple rings, at least {min_len} each"),
    };
    let spouses = if config.exclude_spouses {
        "spouses kept apart"
    } else {
        "spouses allowed"
    };
    let repeats = match config.avoid_repeat_years {
        Some(1) => "no repeat of last year's receiver".to_string(),
        Some(n) => format!("no repeat from the last {n} years"),
        None => "repeats allowed".to_string(),
    };

    rsx! {
        section { class: "section",
            div { class: "section-head", h2 { "How it was drawn" } }
            div { class: "notice", "{mode} · {spouses} · {repeats}" }
        }
    }
}
