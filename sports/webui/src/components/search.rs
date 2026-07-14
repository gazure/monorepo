use dioxus::prelude::*;

use crate::{app::Route, dto::TeamRef, server};

/// Navbar omnibox: search players and teams from anywhere. Enter jumps to
/// the top hit, Escape clears.
#[component]
pub fn GlobalSearch() -> Element {
    let mut q = use_signal(String::new);
    let teams = use_resource(server::team_options);
    let players = use_resource(move || {
        let query = q();
        async move {
            if query.trim().len() < 2 {
                return Ok(Vec::new());
            }
            server::search_players(query, 6).await
        }
    });
    let nav = use_navigator();

    let query_now = q.read().trim().to_lowercase();
    let team_hits: Vec<TeamRef> = if query_now.len() < 2 {
        Vec::new()
    } else {
        match &*teams.read() {
            Some(Ok(all)) => all
                .iter()
                .filter(|t| t.code.to_lowercase().contains(&query_now) || t.name.to_lowercase().contains(&query_now))
                .take(3)
                .cloned()
                .collect(),
            _ => Vec::new(),
        }
    };
    let player_hits = match &*players.read() {
        Some(Ok(hits)) => hits.clone(),
        _ => Vec::new(),
    };
    let open = query_now.len() >= 2 && (!team_hits.is_empty() || !player_hits.is_empty());

    let first_route: Option<Route> = team_hits
        .first()
        .map(|t| Route::TeamDetail { id: t.id })
        .or_else(|| player_hits.first().map(|p| Route::PlayerDetail { id: p.id }));

    rsx! {
        div { class: "omni",
            input {
                r#type: "search",
                placeholder: "Search players & teams…",
                value: "{q}",
                oninput: move |e| q.set(e.value()),
                onkeydown: {
                    let first_route = first_route.clone();
                    move |e| match e.key() {
                        Key::Enter => {
                            if let Some(route) = first_route.clone() {
                                q.set(String::new());
                                nav.push(route);
                            }
                        }
                        Key::Escape => q.set(String::new()),
                        _ => {}
                    }
                },
            }
            if open {
                div { class: "omni-results",
                    for t in team_hits {
                        button {
                            class: "omni-hit",
                            key: "t{t.id}",
                            onclick: move |_| {
                                q.set(String::new());
                                nav.push(Route::TeamDetail { id: t.id });
                            },
                            span { class: "omni-kind", "team" }
                            "{t.code} · {t.name}"
                        }
                    }
                    for p in player_hits {
                        button {
                            class: "omni-hit",
                            key: "p{p.id}",
                            onclick: move |_| {
                                q.set(String::new());
                                nav.push(Route::PlayerDetail { id: p.id });
                            },
                            span { class: "omni-kind", "player" }
                            "{p.name}"
                        }
                    }
                }
            }
        }
    }
}
