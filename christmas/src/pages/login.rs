use dioxus::prelude::*;

/// A plain HTML form posting to `/auth/login`.
///
/// Deliberately not a server function: a native form POST means no JavaScript
/// is involved, and the server can hand back an `HttpOnly` cookie that page
/// scripts cannot read.
#[component]
pub fn Login(next: Option<String>, error: Option<i32>) -> Element {
    let next = next.unwrap_or_else(|| "/".to_string());
    let failed = error.is_some();

    rsx! {
        div { class: "login-shell",
            div { class: "login-card",
                p { class: "eyebrow", "The Exchange" }
                h1 { "Come in" }
                p { class: "muted",
                    "Ask whoever runs the draw for the password."
                }

                if failed {
                    div { class: "error-box", "That password didn't work. Try again." }
                }

                form { method: "post", action: "/auth/login",
                    input { r#type: "hidden", name: "next", value: "{next}" }
                    input {
                        r#type: "password",
                        name: "password",
                        placeholder: "Password",
                        autofocus: true,
                        autocomplete: "current-password",
                        required: true,
                    }
                    button { r#type: "submit", "Sign in" }
                }
            }
        }
    }
}
