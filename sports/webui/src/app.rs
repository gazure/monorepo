use dioxus::prelude::*;

use crate::pages::{
    GameDetail, Games, Home, Leaderboards, Matchup, PlayerDetail, Players, SeasonDetail, Seasons, SqlConsole,
    TeamDetail, Teams,
};

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[layout(Navbar)]
    #[route("/")]
    Home {},
    #[route("/games")]
    Games {},
    #[route("/games/:id")]
    GameDetail { id: i32 },
    #[route("/players")]
    Players {},
    #[route("/players/:id")]
    PlayerDetail { id: i32 },
    #[route("/teams")]
    Teams {},
    #[route("/teams/:id")]
    TeamDetail { id: i32 },
    #[route("/seasons")]
    Seasons {},
    #[route("/seasons/:year")]
    SeasonDetail { year: i32 },
    #[route("/leaderboards?:season")]
    Leaderboards { season: Option<i32> },
    #[route("/matchup?:batter&:pitcher")]
    Matchup { batter: Option<i32>, pitcher: Option<i32> },
    #[route("/sql")]
    SqlConsole {},
}

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");

#[component]
pub fn App() -> Element {
    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        Router::<Route> {}
    }
}

#[component]
fn Navbar() -> Element {
    rsx! {
        nav { class: "navbar",
            Link { to: Route::Home {}, class: "navbar-brand", "⚾ Sports Explorer" }
            div { class: "navbar-links",
                Link { to: Route::Games {}, active_class: "active", "Games" }
                Link { to: Route::Players {}, active_class: "active", "Players" }
                Link { to: Route::Teams {}, active_class: "active", "Teams" }
                Link { to: Route::Seasons {}, active_class: "active", "Seasons" }
                Link {
                    to: Route::Leaderboards { season: None },
                    active_class: "active",
                    "Leaderboards"
                }
                Link {
                    to: Route::Matchup {
                        batter: None,
                        pitcher: None,
                    },
                    active_class: "active",
                    "Matchup"
                }
                Link { to: Route::SqlConsole {}, active_class: "active", "SQL" }
            }
        }
        main { class: "content", Outlet::<Route> {} }
    }
}
