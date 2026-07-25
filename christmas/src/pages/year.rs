use dioxus::prelude::*;

use crate::{components::CycleBoard, server};

#[component]
pub fn YearPage(year: i32) -> Element {
    let draws = use_resource(move || server::list_year(year));

    rsx! {
        header { class: "hero",
            div { class: "hero-copy",
                p { class: "eyebrow", "Looking back" }
                h1 { "Christmas {year}" }
            }
        }

        match &*draws.read() {
            Some(Ok(draws)) if draws.is_empty() => rsx! {
                div { class: "empty-cta",
                    strong { "Nothing drawn in {year}" }
                    "Pick another year from History."
                }
            },
            Some(Ok(draws)) => rsx! {
                for draw in draws.iter() {
                    section { key: "{draw.id}", class: "section",
                        div { class: "section-head",
                            h2 { "{draw.pool_name}" }
                            if let Some(letter) = draw.letter {
                                span { class: "badge current", "letter {letter}" }
                            }
                            span { class: "count", "{draw.participants.len()} people" }
                        }
                        CycleBoard { cycles: draw.cycles(), letter: draw.letter }
                    }
                }
            },
            Some(Err(e)) => rsx! { div { class: "error-box", "{e}" } },
            None => rsx! { div { class: "loading", "Loading {year}…" } },
        }
    }
}
