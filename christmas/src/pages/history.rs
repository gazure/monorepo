use dioxus::prelude::*;

use crate::{app::Route, auth::Role, model::Exchange, server};

#[component]
pub fn History() -> Element {
    let exchanges = use_resource(move || server::list_exchanges(None));

    rsx! {
        header { class: "hero",
            div { class: "hero-copy",
                p { class: "eyebrow", "Every draw ever" }
                h1 { "History" }
                p { "Re-running a draw never erases the old one — earlier revisions stay on the record." }
            }
        }

        match &*exchanges.read() {
            Some(Ok(list)) if list.is_empty() => rsx! {
                div { class: "empty-cta",
                    strong { "Nothing drawn yet" }
                    "The first draw will show up here."
                }
            },
            Some(Ok(list)) => rsx! { HistoryTable { exchanges: list.clone() } },
            Some(Err(e)) => rsx! { div { class: "error-box", "{e}" } },
            None => rsx! { div { class: "loading", "Loading history…" } },
        }
    }
}

#[component]
fn HistoryTable(exchanges: Vec<Exchange>) -> Element {
    // Superseded draws are already filtered out server-side for viewers; the
    // column would only advertise that something is being kept from them.
    let role = use_resource(server::my_role);
    let show_revisions = matches!(&*role.read(), Some(Ok(Role::Manager)));

    // Already ordered year DESC, revision DESC.
    let mut years: Vec<i32> = exchanges.iter().map(|e| e.year).collect();
    years.dedup();

    rsx! {
        for year in years {
            section { key: "y{year}", class: "section",
                div { class: "section-head",
                    h2 { "{year}" }
                    Link { to: Route::YearPage { year }, class: "count", "view rings →" }
                }
                div { class: "table-scroll",
                    table { class: "data-table",
                        thead {
                            tr {
                                th { "Pool" }
                                th { "Letter" }
                                th { "People" }
                                th { "Rings" }
                                if show_revisions {
                                    th { "Revision" }
                                }
                            }
                        }
                        tbody {
                            for ex in exchanges.iter().filter(|e| e.year == year) {
                                tr { key: "{ex.id}",
                                    td {
                                        Link { to: Route::PoolPage { slug: ex.pool_slug.clone() }, "{ex.pool_name}" }
                                    }
                                    td { {ex.letter.map_or_else(|| "—".to_string(), |c| c.to_string())} }
                                    td { class: "mono", "{ex.participants.len()}" }
                                    td { class: "mono", "{ex.cycles().len()}" }
                                    if show_revisions {
                                        td {
                                            if ex.revision == 1 {
                                                span { class: "muted mono", "1" }
                                            } else {
                                                span { class: "badge", "{ex.revision}" }
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
