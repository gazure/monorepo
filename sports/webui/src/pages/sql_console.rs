use dioxus::prelude::*;

use crate::{dto::SqlResult, server};

const DEFAULT_QUERY: &str = "SELECT g.game_date, ta.code AS away, g.away_score, th.code AS home, g.home_score\nFROM games g\nJOIN teams ta ON ta.id = g.away_team_id\nJOIN teams th ON th.id = g.home_team_id\nORDER BY g.game_date DESC\nLIMIT 25";

#[component]
pub fn SqlConsole() -> Element {
    let mut sql = use_signal(|| DEFAULT_QUERY.to_string());
    let mut result = use_signal(|| None::<Result<SqlResult, String>>);
    let mut running = use_signal(|| false);

    let run = move || {
        if running() {
            return;
        }
        let q = sql();
        spawn(async move {
            running.set(true);
            let r = server::run_sql(q).await.map_err(|e| e.to_string());
            result.set(Some(r));
            running.set(false);
        });
    };

    rsx! {
        h1 { "SQL console" }
        div { class: "muted",
            "Read-only: queries run in a READ ONLY transaction with a 5s timeout; results cap at 1000 rows."
        }
        textarea {
            class: "sql-editor",
            spellcheck: "false",
            value: "{sql}",
            oninput: move |e| sql.set(e.value()),
            onkeydown: move |e: Event<KeyboardData>| {
                if e.key() == Key::Enter && (e.modifiers().ctrl() || e.modifiers().meta()) {
                    e.prevent_default();
                    run();
                }
            },
        }
        div { class: "sql-toolbar",
            button { disabled: running(), onclick: move |_| run(), "Run (⌘⏎)" }
            if let Some(Ok(res)) = &*result.read() {
                span { class: "sql-meta",
                    "{res.row_count} rows in {res.elapsed_ms} ms"
                    if res.truncated {
                        " (truncated to 1000)"
                    }
                }
            }
        }
        match &*result.read() {
            Some(Ok(res)) if res.columns.is_empty() => rsx! {
                div { class: "muted", "Query returned no rows." }
            },
            Some(Ok(res)) => rsx! {
                ResultTable { result: res.clone() }
            },
            Some(Err(e)) => rsx! {
                div { class: "sql-error", "{e}" }
            },
            None => rsx! {},
        }
    }
}

#[component]
fn ResultTable(result: SqlResult) -> Element {
    rsx! {
        div { class: "table-scroll",
            table { class: "data-table",
                thead {
                    tr {
                        for col in result.columns.clone() {
                            th { "{col}" }
                        }
                    }
                }
                tbody {
                    for (i , row) in result.rows.clone().into_iter().enumerate() {
                        tr { key: "{i}",
                            for cell in row {
                                td {
                                    match cell {
                                        Some(v) => rsx! {
                                        "{v}"
                                        },
                                        None => rsx! {
                                            span { class: "null-cell", "NULL" }
                                        },
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
