use dioxus::prelude::*;

use crate::{
    components::{Snowfall, StringLights},
    pages::{Admin, History, Home, Login, PoolPage, Reveal, YearPage},
};

const MAIN_CSS: Asset = asset!("/assets/main.css");

/// A gold bauble, inline so the tab icon costs no extra request.
const FAVICON: &str = "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 32 32'%3E\
%3Cpath d='M16 1v4' stroke='%236b8478' stroke-width='1.5'/%3E\
%3Crect x='13' y='5' width='6' height='4' rx='1' fill='%239a9486'/%3E\
%3Ccircle cx='16' cy='20' r='11' fill='%23d4a94e'/%3E\
%3Ccircle cx='12' cy='16' r='3.5' fill='%23f0d79a' opacity='.55'/%3E%3C/svg%3E";

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
        document::Link { rel: "icon", href: FAVICON }
        document::Meta { name: "viewport", content: "width=device-width, initial-scale=1" }
        // Outside the router so it keeps falling across every route, including
        // the ones that opt out of the shell.
        Snowfall {}
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
        StringLights {}
        main { class: "content", Outlet::<Route> {} }
        footer { class: "site-foot", "Drawn fresh every December." }
    }
}
