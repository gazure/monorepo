use dioxus::prelude::*;
use dioxus_router::{Link, Outlet, Routable};

use crate::{
    app::{
        debug_logs::DebugLogs, draft_details::DraftDetails, drafts::Drafts, error_logs::ErrorLogs,
        match_details::MatchDetails, matches::Matches,
    },
    backend::{SharedAuthState, SharedUpdateState},
};

fn open_github() {
    if let Err(e) = open::that("https://github.com/gazure/monorepo") {
        tracingx::error!("Failed to open URL: {}", e);
    }
}

#[derive(Clone, Routable, Debug, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[layout(Layout)]
        #[route("/")]
        Home {},
        #[route("/matches")]
        Matches {},
        #[route("/errors")]
        ErrorLogs {},
        #[route("/contact")]
        Contact {},
        #[route("/match/:id")]
        MatchDetails{ id: String },
        #[route("/drafts")]
        Drafts {},
        #[route("/drafts/:id")]
        DraftDetails { id: String },
        #[route("/debug")]
        DebugLogs {},
    #[end_layout]
    #[route("/:..route")]
    PageNotFound { route: Vec<String> },
}

#[component]
fn Home() -> Element {
    rsx! {
        div { class: "bg-white rounded-lg shadow-md p-6",
            h1 { class: "text-2xl font-bold mb-4 text-gray-800", "Home Page" }
            p { class: "text-gray-600",
                "Welcome to ArenaBuddy. Track and analyze your Arena matches."
            }
        }
    }
}

#[component]
fn Contact() -> Element {
    rsx! {
        div { class: "bg-white rounded-lg shadow-md p-6",
            h1 { class: "text-2xl font-bold mb-4 text-gray-800", "Contact" }
            a {
                href: "#",
                onclick: move |_| open_github(),
                class: "text-blue-600 hover:text-blue-800 transition-colors duration-200",
                "Github Repo"
            }
        }
    }
}

#[component]
fn PageNotFound(route: Vec<String>) -> Element {
    rsx! {
        div { class: "text-center mt-8",
            h1 { class: "text-2xl font-bold text-red-600", "Page Not Found" }
            p { class: "mt-2 text-gray-600",
                "The page you're looking for doesn't exist."
            }
        }
    }
}

#[component]
fn Layout() -> Element {
    let auth_state = use_context::<SharedAuthState>();
    let update_state = use_context::<SharedUpdateState>();
    let mut login_status = use_signal(|| None::<String>);
    let mut login_loading = use_signal(|| false);
    let mut update_version = use_signal(|| None::<String>);
    let mut update_msg = use_signal(|| None::<(String, &'static str)>); // (message, color class)

    // Check current auth state on render
    let auth_state_effect = auth_state.clone();
    use_effect(move || {
        let auth_state = auth_state_effect.clone();
        spawn(async move {
            let state = auth_state.lock().await;
            login_status.set(state.as_ref().map(|s| s.user.username.clone()));
        });
    });

    // Poll update state
    let update_state_effect = update_state.clone();
    use_effect(move || {
        let update_state = update_state_effect.clone();
        spawn(async move {
            let state = update_state.lock().await;
            match &*state {
                crate::backend::update::UpdateStatus::Available { version } => {
                    update_version.set(Some(version.clone()));
                    update_msg.set(None);
                }
                crate::backend::update::UpdateStatus::RestartRequired => {
                    update_version.set(None);
                    update_msg.set(Some(("Update installed — restart to apply".into(), "text-green-400")));
                }
                crate::backend::update::UpdateStatus::Updating => {
                    update_version.set(None);
                    update_msg.set(Some(("Updating...".into(), "text-yellow-400")));
                }
                crate::backend::update::UpdateStatus::Error(e) => {
                    update_version.set(None);
                    update_msg.set(Some((format!("Update error: {e}"), "text-red-400")));
                }
                _ => {
                    update_version.set(None);
                    update_msg.set(None);
                }
            }
        });
    });

    let on_update = move |_| {
        let update_state = update_state.clone();
        spawn(async move {
            crate::backend::update::apply_update(update_state).await;
        });
    };

    let on_login = move |_| {
        let auth_state = auth_state.clone();
        spawn(async move {
            let Ok(grpc_url) = std::env::var("ARENABUDDY_GRPC_URL") else {
                tracingx::error!("ARENABUDDY_GRPC_URL not set, cannot login");
                return;
            };
            let Ok(client_id) = std::env::var("DISCORD_CLIENT_ID") else {
                tracingx::error!("DISCORD_CLIENT_ID not set, cannot login");
                return;
            };

            login_loading.set(true);
            match crate::backend::auth::login(&grpc_url, &client_id).await {
                Ok(state) => {
                    let username = state.user.username.clone();
                    *auth_state.lock().await = Some(state);
                    login_status.set(Some(username));
                }
                Err(e) => {
                    tracingx::error!("Login failed: {e}");
                }
            }
            login_loading.set(false);
        });
    };

    rsx! {
        nav { class: "bg-gray-800 p-4 shadow-md",
            div { class: "container mx-auto flex justify-between items-center",
                ul { class: "flex space-x-6 text-white",
                    li {
                        Link {
                            to: Route::Home {},
                            class: "hover:text-blue-400 transition-colors duration-200",
                            "Home"
                        }
                    }
                    li {
                        Link {
                            to: Route::Matches {},
                            class: "hover:text-blue-400 transition-colors duration-200",
                            "Matches"
                        }
                    }
                    li {
                        Link {
                            to: Route::Drafts { },
                            class: "hover:text-blue-400 transition-colors duration-200",
                            "Drafts"
                        }
                    }
                    li {
                        Link {
                            to: Route::ErrorLogs {},
                            class: "hover:text-blue-400 transition-colors duration-200",
                            "Error Logs"
                        }
                    }
                    li {
                        Link {
                            to: Route::DebugLogs {},
                            class: "hover:text-blue-400 transition-colors duration-200",
                            "Debug Logs"
                        }
                    }
                    li {
                        Link {
                            to: Route::Contact {},
                            class: "hover:text-blue-400 transition-colors duration-200",
                            "Contact"
                        }
                    }
                }
                div { class: "flex items-center space-x-4",
                    if let Some(version) = update_version() {
                        div { class: "flex items-center space-x-2",
                            span { class: "text-yellow-400 text-sm", "v{version} available" }
                            button {
                                class: "bg-yellow-600 hover:bg-yellow-700 text-white text-xs px-2 py-1 rounded transition-colors duration-200",
                                onclick: on_update,
                                "Update now"
                            }
                        }
                    } else if let Some((msg, color)) = update_msg() {
                        span { class: "{color} text-sm", "{msg}" }
                    }
                    div { class: "text-white",
                        if let Some(username) = login_status() {
                            span { class: "text-green-400 text-sm", "Logged in as {username}" }
                        } else if login_loading() {
                            span { class: "text-yellow-400 text-sm", "Logging in..." }
                        } else {
                            button {
                                class: "bg-indigo-600 hover:bg-indigo-700 text-white text-sm px-3 py-1 rounded transition-colors duration-200",
                                onclick: on_login,
                                "Login with Discord"
                            }
                        }
                    }
                }
            }
        }
        main { class: "container mx-auto p-4",
            Outlet::<Route> {}
        }
    }
}
