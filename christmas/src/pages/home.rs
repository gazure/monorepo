use dioxus::prelude::*;

use super::current_year;
use crate::{
    app::Route,
    components::CycleBoard,
    model::{Exchange, Pool},
    server,
};

#[component]
pub fn Home() -> Element {
    let year = current_year();
    let pools = use_resource(server::list_pools);
    let draws = use_resource(move || server::list_year(year));

    rsx! {
        match (&*pools.read(), &*draws.read()) {
            (Some(Ok(pools)), Some(Ok(draws))) => rsx! {
                HomeBody { year, pools: pools.clone(), draws: draws.clone() }
            },
            (Some(Err(e)), _) | (_, Some(Err(e))) => rsx! {
                div { class: "error-box", "Couldn't load the exchange: {e}" }
            },
            _ => rsx! { div { class: "loading", "Loading…" } },
        }
    }
}

#[component]
fn HomeBody(year: i32, pools: Vec<Pool>, draws: Vec<Exchange>) -> Element {
    // The headline letter comes from the biggest pool that has drawn, since
    // letters are chosen per pool and may differ.
    let headline = draws
        .iter()
        .filter(|d| d.letter.is_some())
        .max_by_key(|d| d.participants.len())
        .cloned();

    rsx! {
        header { class: "hero",
            div { class: "hero-copy",
                p { class: "eyebrow", "Christmas {year}" }
                h1 {
                    if draws.is_empty() {
                        "The hat's still empty."
                    } else {
                        "Everyone's got someone."
                    }
                }
                p {
                    if draws.is_empty() {
                        "Once the draw is run, your name and your person will be waiting here — along with the letter for this year."
                    } else {
                        "Find your name in the ring and follow the arrow to whoever you're shopping for. Every gift starts with this year's letter — though people get creative with that."
                    }
                }
            }

            if let Some(head) = headline.as_ref() {
                if let Some(letter) = head.letter {
                    div { class: "letter-mark",
                        div { class: "ornament",
                            div { class: "letter-glyph", "{letter}" }
                        }
                        div { class: "letter-caption", "{head.pool_name} · gifts start here" }
                    }
                }
            }
        }

        section { class: "section",
            div { class: "section-head",
                h2 { "Pools" }
                span { class: "count", "{pools.len()}" }
            }

            if pools.is_empty() {
                div { class: "empty-cta",
                    strong { "No pools yet" }
                    "Create a pool in Manage, add the family, then run the first draw."
                }
            } else {
                div { class: "pool-grid",
                    for pool in pools.iter() {
                        {
                            let draw = draws.iter().find(|d| d.pool_id == pool.id);
                            rsx! {
                                Link {
                                    key: "{pool.id}",
                                    class: "pool-card",
                                    to: Route::PoolPage { slug: pool.slug.clone() },
                                    if let Some(letter) = draw.and_then(|d| d.letter) {
                                        span { class: "pool-card-letter", "{letter}" }
                                    }
                                    h3 { "{pool.name}" }
                                    p {
                                        {pool.description.clone().unwrap_or_else(|| "—".to_string())}
                                    }
                                    div { class: "pool-card-meta",
                                        span { "{pool.member_count} members" }
                                        match draw {
                                            Some(d) => rsx! { span { "{d.pairings.len()} pairings" } },
                                            None => rsx! { span { "not drawn" } },
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        for draw in draws.iter().filter(|d| !d.pairings.is_empty()) {
            section { key: "draw{draw.id}", class: "section",
                div { class: "section-head",
                    h2 { "{draw.pool_name}" }
                    if draw.revision > 1 {
                        span { class: "badge", "revision {draw.revision}" }
                    }
                    span { class: "count", "{draw.participants.len()} people" }
                }
                CycleBoard { cycles: draw.cycles(), letter: draw.letter }
            }
        }
    }
}
