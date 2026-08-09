use dioxus::prelude::*;

use crate::{app::Route, model::Pool, server};

#[component]
pub fn PoolsSection(pools: Vec<Pool>, on_change: EventHandler<()>) -> Element {
    let mut name = use_signal(String::new);
    let mut description = use_signal(String::new);
    let mut error = use_signal(|| None::<String>);

    let create = move |_| {
        let new_name = name();
        if new_name.trim().is_empty() {
            return;
        }
        let desc = description();
        spawn(async move {
            let desc = if desc.trim().is_empty() { None } else { Some(desc) };
            match server::create_pool(new_name, desc).await {
                Ok(_) => {
                    name.set(String::new());
                    description.set(String::new());
                    error.set(None);
                    on_change.call(());
                }
                Err(e) => error.set(Some(e.to_string())),
            }
        });
    };

    rsx! {
        div { class: "panel",
            h3 { "Pools" }

            if let Some(e) = error() {
                div { class: "error-box", "{e}" }
            }

            div { class: "field-row",
                input {
                    r#type: "text",
                    placeholder: "Pool name",
                    value: "{name}",
                    oninput: move |e| name.set(e.value()),
                }
                input {
                    r#type: "text",
                    placeholder: "Short description (optional)",
                    value: "{description}",
                    oninput: move |e| description.set(e.value()),
                }
                button { onclick: create, "Add pool" }
            }

            if pools.is_empty() {
                p { class: "muted", "No pools yet." }
            } else {
                div { class: "table-scroll",
                    table { class: "data-table",
                        thead {
                            tr {
                                th { "Name" }
                                th { "Members" }
                                th { "" }
                            }
                        }
                        tbody {
                            for pool in pools.iter() {
                                {
                                    let id = pool.id;
                                    rsx! {
                                        tr { key: "{id}",
                                            td {
                                                Link {
                                                    to: Route::PoolPage { slug: pool.slug.clone() },
                                                    "{pool.name}"
                                                }
                                            }
                                            td { class: "mono", "{pool.member_count}" }
                                            td {
                                                button {
                                                    class: "danger",
                                                    onclick: move |_| {
                                                        spawn(async move {
                                                            if server::delete_pool(id).await.is_ok() {
                                                                on_change.call(());
                                                            }
                                                        });
                                                    },
                                                    "Remove"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                p { class: "muted", style: "font-size: 0.8rem; margin-bottom: 0",
                    "Removing a pool deletes its draws. Past years already recorded elsewhere are unaffected."
                }
            }
        }
    }
}
