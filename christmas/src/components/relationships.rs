use dioxus::prelude::*;

use crate::{
    model::{Participant, Relationship, RelationshipKind},
    server,
};

#[component]
pub fn RelationshipsSection(
    participants: Vec<Participant>,
    relationships: Vec<Relationship>,
    on_change: EventHandler<()>,
) -> Element {
    let mut a_id = use_signal(|| None::<i32>);
    let mut b_id = use_signal(|| None::<i32>);
    let mut kind = use_signal(|| RelationshipKind::Spouse);
    let mut error = use_signal(|| None::<String>);

    let add = move |_| {
        let (Some(a), Some(b)) = (a_id(), b_id()) else {
            error.set(Some("Pick two people".to_string()));
            return;
        };
        let k = kind();
        spawn(async move {
            match server::add_relationship(a, b, k, None).await {
                Ok(_) => {
                    a_id.set(None);
                    b_id.set(None);
                    error.set(None);
                    on_change.call(());
                }
                Err(e) => error.set(Some(e.to_string())),
            }
        });
    };

    rsx! {
        div { class: "panel",
            h3 { "Relationships" }
            p { class: "muted", style: "font-size: 0.85rem; margin-top: -0.5rem",
                "Spouses and housemates are skipped only when the draw's \"keep spouses apart\" toggle is on. Manual exclusions always apply."
            }

            if let Some(e) = error() {
                div { class: "error-box", "{e}" }
            }

            div { class: "field-row",
                select {
                    value: a_id().map_or_else(String::new, |v| v.to_string()),
                    onchange: move |e| a_id.set(e.value().parse().ok()),
                    option { value: "", "Person…" }
                    for p in participants.iter() {
                        option { key: "{p.id}", value: "{p.id}", "{p.name}" }
                    }
                }
                select {
                    value: b_id().map_or_else(String::new, |v| v.to_string()),
                    onchange: move |e| b_id.set(e.value().parse().ok()),
                    option { value: "", "Person…" }
                    for p in participants.iter() {
                        option { key: "{p.id}", value: "{p.id}", "{p.name}" }
                    }
                }
                select {
                    value: kind().as_db(),
                    onchange: move |e| kind.set(RelationshipKind::from_db(&e.value())),
                    for k in RelationshipKind::ALL {
                        option { key: "{k.as_db()}", value: "{k.as_db()}", "{k.label()}" }
                    }
                }
                button { onclick: add, "Link" }
            }

            if relationships.is_empty() {
                p { class: "muted", "No relationships recorded." }
            } else {
                div { class: "table-scroll",
                    table { class: "data-table",
                        thead {
                            tr {
                                th { "Between" }
                                th { "Kind" }
                                th { "" }
                            }
                        }
                        tbody {
                            for rel in relationships.iter() {
                                {
                                    let id = rel.id;
                                    rsx! {
                                        tr { key: "{id}",
                                            td { "{rel.participant_a.name} & {rel.participant_b.name}" }
                                            td { span { class: "badge", "{rel.kind.label()}" } }
                                            td {
                                                button {
                                                    class: "danger",
                                                    onclick: move |_| {
                                                        spawn(async move {
                                                            if server::remove_relationship(id).await.is_ok() {
                                                                on_change.call(());
                                                            }
                                                        });
                                                    },
                                                    "Unlink"
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
