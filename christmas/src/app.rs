use dioxus::prelude::*;

use crate::pages::{Admin, History, Home, Login, PoolPage, Reveal, YearPage};

const MAIN_CSS: Asset = asset!("/assets/main.css");

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    // Outside the shell: no nav to click when you can't get in yet.
    #[route("/login?:next&:error")]
    Login { next: Option<String>, error: Option<i32> },

    // Outside the shell too: the ceremony is meant to be screen-shared, so it
    // gets the whole viewport with no navigation furniture.
    #[route("/reveal/:id")]
    Reveal { id: i32 },

    #[layout(Shell)]
    #[route("/")]
    Home {},
    #[route("/pools/:slug")]
    PoolPage { slug: String },
    #[route("/years/:year")]
    YearPage { year: i32 },
    #[route("/history")]
    History {},
    #[route("/admin")]
    Admin {},
}

#[component]
pub fn App() -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Meta { name: "viewport", content: "width=device-width, initial-scale=1" }
        Router::<Route> {}
    }
}

#[component]
fn Shell() -> Element {
    rsx! {
        nav { class: "navbar",
            Link { to: Route::Home {}, class: "navbar-brand",
                "The " em { "Exchange" }
            }
            div { class: "navbar-links",
                Link { to: Route::Home {}, active_class: "active", "This year" }
                Link { to: Route::History {}, active_class: "active", "History" }
                Link { to: Route::Admin {}, active_class: "active", "Manage" }
                // A form, not a link: signing out changes state, and this keeps
                // the whole auth flow free of JavaScript.
                form { method: "post", action: "/auth/logout", class: "signout",
                    button { r#type: "submit", class: "linklike", "Sign out" }
                }
            }
        }
        main { class: "content", Outlet::<Route> {} }
        footer { class: "site-foot", "Drawn fresh every December." }
    }
}
