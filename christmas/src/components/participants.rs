use dioxus::prelude::*;

use crate::{
    model::{Participant, Pool},
    server,
};

#[component]
pub fn ParticipantsSection(
    participants: Vec<Participant>,
    pools: Vec<Pool>,
    memberships: Vec<(i32, i32)>,
    on_change: EventHandler<()>,
) -> Element {
    let mut name = use_signal(String::new);
    let mut error = use_signal(|| None::<String>);

    let pool_list = pools.clone();
    let add = move |()| {
        let new_name = name();
        if new_name.trim().is_empty() {
            return;
        }
        spawn(async move {
            match server::add_participant(new_name, Vec::new()).await {
                Ok(_) => {
                    name.set(String::new());
                    error.set(None);
                    on_change.call(());
                }
                Err(e) => error.set(Some(e.to_string())),
            }
        });
    };

    rsx! {
        div { class: "panel",
            h3 { "People" }

            if let Some(e) = error() {
                div { class: "error-box", "{e}" }
            }

            div { class: "field-row",
                input {
                    r#type: "text",
                    placeholder: "Name",
                    value: "{name}",
                    oninput: move |e| name.set(e.value()),
                    onkeydown: move |e| if e.key() == Key::Enter { add(()) },
                }
                button { onclick: move |_| add(()), "Add person" }
            }

            if participants.is_empty() {
                p { class: "muted", "Nobody yet." }
            } else {
                div { class: "table-scroll",
                    table { class: "data-table",
                        thead {
                            tr {
                                th { "Name" }
                                th { "Pools" }
                                th { "" }
                            }
                        }
                        tbody {
                            for person in participants.iter() {
                                {
                                    let person_id = person.id;
                                    let pools_for_row = pool_list.clone();
                                    let memberships = memberships.clone();
                                    rsx! {
                                        tr { key: "{person_id}",
                                            td { "{person.name}" }
                                            td {
                                                div { class: "field-row", style: "margin: 0; gap: 0.75rem",
                                                    for pool in pools_for_row.iter() {
                                                        {
                                                            let pool_id = pool.id;
                                                            let member = memberships.contains(&(pool_id, person_id));
                                                            rsx! {
                                                                label {
                                                                    key: "{pool_id}",
                                                                    style: "display: flex; align-items: center; gap: 0.3rem",
                                                                    input {
                                                                        r#type: "checkbox",
                                                                        checked: member,
                                                                        onchange: move |_| {
                                                                            spawn(async move {
                                                                                let result = if member {
                                                                                    server::remove_member(pool_id, person_id).await
                                                                                } else {
                                                                                    server::add_member(pool_id, person_id).await
                                                                                };
                                                                                if result.is_ok() {
                                                                                    on_change.call(());
                                                                                }
                                                                            });
                                                                        },
                                                                    }
                                                                    "{pool.name}"
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            td {
                                                button {
                                                    class: "danger",
                                                    onclick: move |_| {
                                                        spawn(async move {
                                                            if server::remove_participant(person_id).await.is_ok() {
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
            }
        }
    }
}
