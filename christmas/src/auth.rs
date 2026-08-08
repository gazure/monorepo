//! Two shared passwords: one for the family, one for whoever runs the draw.
//!
//! Enforcement lives in one axum middleware wrapping the whole router rather
//! than in each server function, so a newly added endpoint cannot silently ship
//! unprotected — unknown `/api` paths require the manager role by default.

use serde::{Deserialize, Serialize};

/// Who someone is allowed to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Can read the exchange.
    Viewer,
    /// Can also change pools, people, and run draws.
    Manager,
}

impl Role {
    /// Managers can do anything a viewer can.
    pub fn satisfies(self, required: Self) -> bool {
        self >= required
    }
}

/// What a given request path demands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    Public,
    Viewer,
    Manager,
}

pub const COOKIE_NAME: &str = "christmas_session";

/// The only server functions a viewer may call.
///
/// Named individually rather than matched by a `list_` prefix: the manage page
/// reads people, relationships, memberships and excluded letters through
/// `list_*` functions too, and a prefix rule handed all of that to anyone with
/// the family password.
const VIEWER_ENDPOINTS: [&str; 6] = [
    "list_pools",
    "list_year",
    "list_exchanges",
    "pool_detail",
    "exchange_detail",
    "my_role",
];

/// Recovers a server function's name from its request path.
///
/// Dioxus appends a decimal hash to each endpoint (`list_pools117422…`), so
/// trimming the trailing digits gives the name back and lets the allowlist
/// match exactly. Exactness is the point: as a prefix, `list_exchanges` is one
/// typo away from also admitting `list_excluded_letters`.
fn server_fn_name(endpoint: &str) -> &str {
    endpoint.trim_end_matches(|c: char| c.is_ascii_digit())
}

/// Classifies a request path.
///
/// Deliberately fails closed: anything under `/api` that is not on the viewer
/// allowlist requires a manager, so adding a server function without thinking
/// about auth denies access rather than granting it.
pub fn required_access(path: &str) -> Access {
    if path == "/login"
        || path.starts_with("/auth/")
        || path.starts_with("/assets/")
        || path.starts_with("/wasm/")
        || path.starts_with("/_dioxus")
        || path == "/favicon.ico"
    {
        return Access::Public;
    }

    if let Some(endpoint) = path.strip_prefix("/api/") {
        if VIEWER_ENDPOINTS.contains(&server_fn_name(endpoint)) {
            return Access::Viewer;
        }
        return Access::Manager;
    }

    if path == "/admin" {
        return Access::Manager;
    }

    Access::Viewer
}

/// Pulls our session token out of a `Cookie` header value.
pub fn token_from_cookie_header(header: &str) -> Option<&str> {
    header.split(';').find_map(|part| {
        let part = part.trim();
        let (name, value) = part.split_once('=')?;
        (name.trim() == COOKIE_NAME).then(|| value.trim())
    })
}

/// Length-independent equality, so a wrong password cannot be narrowed down by
/// timing how long the comparison took.
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(feature = "server")]
pub use server_impl::*;

#[cfg(feature = "server")]
mod server_impl {
    use std::{
        collections::HashMap,
        sync::{Mutex, OnceLock},
        time::{Duration, SystemTime},
    };

    use axum::{
        extract::{Form, Request},
        http::{HeaderValue, StatusCode, header},
        middleware::Next,
        response::{IntoResponse, Response},
    };

    use super::{Access, COOKIE_NAME, Role, constant_time_eq, required_access, token_from_cookie_header};

    /// How long a login lasts. Long, because this is a family site that gets
    /// visited a handful of times each December.
    const SESSION_TTL: Duration = Duration::from_hours(24 * 30);

    struct Session {
        role: Role,
        expires: SystemTime,
    }

    /// In memory, so a redeploy signs everyone out. Acceptable here, and it
    /// avoids another table and another thing to migrate.
    static SESSIONS: OnceLock<Mutex<HashMap<String, Session>>> = OnceLock::new();

    fn sessions() -> &'static Mutex<HashMap<String, Session>> {
        SESSIONS.get_or_init(|| Mutex::new(HashMap::new()))
    }

    fn view_password() -> Option<String> {
        std::env::var("CHRISTMAS_VIEW_PASSWORD").ok().filter(|p| !p.is_empty())
    }

    fn admin_password() -> Option<String> {
        std::env::var("CHRISTMAS_ADMIN_PASSWORD").ok().filter(|p| !p.is_empty())
    }

    /// Auth is off entirely when neither password is configured, so `dx serve`
    /// stays frictionless. Boot logs a warning when that happens.
    pub fn enabled() -> bool {
        view_password().is_some() || admin_password().is_some()
    }

    /// Logs the auth posture once at startup.
    pub fn report_configuration() {
        if !enabled() {
            tracingx::warn!("no CHRISTMAS_VIEW_PASSWORD or CHRISTMAS_ADMIN_PASSWORD set — the site is completely open");
            return;
        }
        if view_password().is_none() {
            tracingx::warn!("CHRISTMAS_VIEW_PASSWORD is not set — only the manager password can sign in");
        }
        if admin_password().is_none() {
            tracingx::warn!("CHRISTMAS_ADMIN_PASSWORD is not set — nobody can reach the manage page");
        }
        tracingx::info!("password protection enabled");
    }

    /// Highest role the supplied password unlocks.
    fn role_for_password(candidate: &str) -> Option<Role> {
        // Check both, without short-circuiting, so timing does not reveal which
        // password was closer.
        let is_admin = admin_password().is_some_and(|p| constant_time_eq(p.as_bytes(), candidate.as_bytes()));
        let is_viewer = view_password().is_some_and(|p| constant_time_eq(p.as_bytes(), candidate.as_bytes()));

        if is_admin {
            Some(Role::Manager)
        } else if is_viewer {
            Some(Role::Viewer)
        } else {
            None
        }
    }

    /// 256 bits from the OS-seeded generator — `fastrand`, used for draws, is a
    /// plain PRNG and must not be used for session tokens.
    fn new_token() -> String {
        use std::fmt::Write as _;

        let bytes: [u8; 32] = rand::random();
        bytes.iter().fold(String::with_capacity(64), |mut acc, b| {
            let _ = write!(acc, "{b:02x}");
            acc
        })
    }

    fn role_for_token(token: &str) -> Option<Role> {
        let mut store = sessions().lock().ok()?;
        let now = SystemTime::now();
        store.retain(|_, s| s.expires > now);
        store.get(token).map(|s| s.role)
    }

    /// The role of whoever is calling the current server function.
    ///
    /// Server functions run behind the same middleware, so this only re-reads
    /// what the guard already validated. With auth switched off entirely,
    /// everyone is a manager — that is what makes `dx serve` usable.
    pub async fn caller_role() -> Role {
        if !enabled() {
            return Role::Manager;
        }

        let Ok(headers) = dioxus::fullstack::FullstackContext::extract::<axum::http::HeaderMap, _>().await else {
            return Role::Viewer;
        };

        headers
            .get(header::COOKIE)
            .and_then(|h| h.to_str().ok())
            .and_then(token_from_cookie_header)
            .and_then(role_for_token)
            .unwrap_or(Role::Viewer)
    }

    /// The role a request carries, or `None` if it is not signed in.
    fn request_role(req: &Request) -> Option<Role> {
        let header = req.headers().get(header::COOKIE)?.to_str().ok()?;
        let token = token_from_cookie_header(header)?;
        role_for_token(token)
    }

    /// Cookies must not carry `Secure` over plain http or the browser drops
    /// them, which would silently break local development.
    fn host_is_local(headers: &axum::http::HeaderMap) -> bool {
        headers
            .get(header::HOST)
            .and_then(|h| h.to_str().ok())
            .is_some_and(|host| host.starts_with("localhost") || host.starts_with("127.0.0.1"))
    }

    fn session_cookie(token: &str, local: bool) -> String {
        let secure = if local { "" } else { " Secure;" };
        format!(
            "{COOKIE_NAME}={token}; Path=/; HttpOnly; SameSite=Lax;{secure} Max-Age={}",
            SESSION_TTL.as_secs()
        )
    }

    /// The guard. Wraps the entire router.
    pub async fn guard(req: Request, next: Next) -> Response {
        if !enabled() {
            return next.run(req).await;
        }

        let path = req.uri().path().to_string();
        let required = required_access(&path);
        if required == Access::Public {
            return next.run(req).await;
        }

        let role = request_role(&req);
        let allowed = match (role, required) {
            (Some(have), Access::Viewer) => have.satisfies(Role::Viewer),
            (Some(have), Access::Manager) => have.satisfies(Role::Manager),
            (Some(_), Access::Public) | (None, _) => false,
        };

        if allowed {
            return next.run(req).await;
        }

        // API callers get a status they can act on; browsers get sent to the
        // login page with somewhere to return to.
        if path.starts_with("/api/") {
            let status = if role.is_some() {
                StatusCode::FORBIDDEN
            } else {
                StatusCode::UNAUTHORIZED
            };
            return (status, "Not signed in").into_response();
        }

        let target = format!("/login?next={}", urlencode(&path));
        (
            StatusCode::SEE_OTHER,
            [(
                header::LOCATION,
                HeaderValue::from_str(&target).unwrap_or(HeaderValue::from_static("/login")),
            )],
        )
            .into_response()
    }

    /// Minimal percent-encoding for the `next` parameter.
    fn urlencode(value: &str) -> String {
        value
            .bytes()
            .map(|b| match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => char::from(b).to_string(),
                other => format!("%{other:02X}"),
            })
            .collect()
    }

    #[derive(serde::Deserialize)]
    pub struct LoginForm {
        password: String,
        #[serde(default)]
        next: String,
    }

    /// `POST /auth/login` — a plain form target, so the login page needs no
    /// JavaScript and the cookie can stay `HttpOnly`.
    pub async fn login(req_headers: axum::http::HeaderMap, Form(form): Form<LoginForm>) -> Response {
        let local = host_is_local(&req_headers);

        let Some(role) = role_for_password(&form.password) else {
            // Slow failures down a little so the passwords cannot be guessed
            // quickly, without holding a real user up noticeably.
            tokio::time::sleep(Duration::from_millis(400)).await;
            tracingx::warn!("failed sign-in attempt");
            return redirect_to("/login?error=1");
        };

        let token = new_token();
        if let Ok(mut store) = sessions().lock() {
            store.insert(
                token.clone(),
                Session {
                    role,
                    expires: SystemTime::now() + SESSION_TTL,
                },
            );
        }
        tracingx::info!(?role, "sign-in");

        // Only ever bounce back to a path on this site.
        let next = if form.next.starts_with('/') && !form.next.starts_with("//") {
            form.next.clone()
        } else {
            "/".to_string()
        };

        let cookie = session_cookie(&token, local);
        let mut response = redirect_to(&next);
        if let Ok(value) = HeaderValue::from_str(&cookie) {
            response.headers_mut().insert(header::SET_COOKIE, value);
        }
        response
    }

    /// `POST /auth/logout`
    #[expect(clippy::unused_async, reason = "axum handlers must be async")]
    pub async fn logout(req_headers: axum::http::HeaderMap) -> Response {
        if let Some(token) = req_headers
            .get(header::COOKIE)
            .and_then(|h| h.to_str().ok())
            .and_then(token_from_cookie_header)
            && let Ok(mut store) = sessions().lock()
        {
            store.remove(token);
        }

        let cleared = format!("{COOKIE_NAME}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0");
        let mut response = redirect_to("/login");
        if let Ok(value) = HeaderValue::from_str(&cleared) {
            response.headers_mut().insert(header::SET_COOKIE, value);
        }
        response
    }

    /// A 303 to a path on this site. Falls back to the root if the path cannot
    /// be represented as a header value.
    fn redirect_to(path: &str) -> Response {
        let location = HeaderValue::from_str(path).unwrap_or(HeaderValue::from_static("/"));
        (StatusCode::SEE_OTHER, [(header::LOCATION, location)]).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_only_endpoints_are_viewer_visible() {
        assert_eq!(required_access("/api/list_pools123"), Access::Viewer);
        assert_eq!(required_access("/api/list_exchanges99"), Access::Viewer);
        assert_eq!(required_access("/api/pool_detail42"), Access::Viewer);
        // Explicitly opted in so the reveal page can be shared with the family.
        assert_eq!(required_access("/api/exchange_detail7"), Access::Viewer);
        // Everyone needs to be able to ask what they are.
        assert_eq!(required_access("/api/my_role99"), Access::Viewer);
    }

    #[test]
    fn mutating_endpoints_require_a_manager() {
        for path in [
            "/api/run_draw1",
            "/api/record_past_draw1",
            "/api/preview_swap1",
            "/api/apply_swap1",
            "/api/restore_revision1",
            "/api/create_pool1",
            "/api/delete_pool1",
            "/api/add_participant1",
            "/api/remove_member1",
            "/api/set_excluded_letters1",
            "/api/add_relationship1",
        ] {
            assert_eq!(required_access(path), Access::Manager, "{path} must be manager-only");
        }
    }

    /// The manage page reads through `list_*` too. A prefix rule handed the
    /// whole roster — and everyone's relationships — to the family password.
    #[test]
    fn reads_only_the_manage_page_needs_require_a_manager() {
        for path in [
            "/api/list_participants1",
            "/api/list_relationships1",
            "/api/list_memberships1",
            "/api/list_excluded_letters1",
            "/api/list_all_excluded_letters1",
        ] {
            assert_eq!(required_access(path), Access::Manager, "{path} must be manager-only");
        }
    }

    /// `list_exchanges` is viewer-visible and `list_excluded_letters` is not,
    /// and they share seven characters of prefix.
    #[test]
    fn the_allowlist_matches_whole_names_not_prefixes() {
        assert_eq!(required_access("/api/list_exchanges1"), Access::Viewer);
        assert_eq!(required_access("/api/list_excluded_letters1"), Access::Manager);
        assert_eq!(required_access("/api/list_pools_and_secrets1"), Access::Manager);
    }

    #[test]
    fn the_hash_suffix_is_not_part_of_the_name() {
        assert_eq!(server_fn_name("list_pools11742215028865194505"), "list_pools");
        assert_eq!(server_fn_name("my_role"), "my_role");
    }

    /// The property that makes this safe to extend.
    #[test]
    fn unknown_api_endpoints_fail_closed() {
        assert_eq!(required_access("/api/something_new_entirely"), Access::Manager);
        assert_eq!(required_access("/api/"), Access::Manager);
    }

    #[test]
    fn the_manage_page_requires_a_manager() {
        assert_eq!(required_access("/admin"), Access::Manager);
    }

    #[test]
    fn pages_need_a_viewer() {
        for path in ["/", "/history", "/pools/pets", "/years/2025", "/reveal/3"] {
            assert_eq!(required_access(path), Access::Viewer);
        }
    }

    #[test]
    fn login_and_static_assets_are_public() {
        for path in [
            "/login",
            "/auth/login",
            "/auth/logout",
            "/assets/main-abc.css",
            "/wasm/christmas.js",
            "/favicon.ico",
        ] {
            assert_eq!(required_access(path), Access::Public, "{path} must be reachable");
        }
    }

    #[test]
    fn managers_can_do_anything_viewers_can() {
        assert!(Role::Manager.satisfies(Role::Viewer));
        assert!(Role::Manager.satisfies(Role::Manager));
        assert!(Role::Viewer.satisfies(Role::Viewer));
        assert!(!Role::Viewer.satisfies(Role::Manager));
    }

    #[test]
    fn finds_our_cookie_among_others() {
        assert_eq!(
            token_from_cookie_header("theme=dark; christmas_session=abc123; other=1"),
            Some("abc123")
        );
        assert_eq!(token_from_cookie_header("christmas_session=solo"), Some("solo"));
        assert_eq!(token_from_cookie_header("unrelated=1"), None);
        assert_eq!(token_from_cookie_header(""), None);
    }

    /// A cookie whose name merely ends in ours must not be mistaken for it.
    #[test]
    fn does_not_match_a_similarly_named_cookie() {
        assert_eq!(token_from_cookie_header("not_christmas_session=abc"), None);
    }

    #[test]
    fn constant_time_eq_still_compares_correctly() {
        assert!(constant_time_eq(b"hunter2", b"hunter2"));
        assert!(!constant_time_eq(b"hunter2", b"hunter3"));
        assert!(!constant_time_eq(b"short", b"longer"));
        assert!(constant_time_eq(b"", b""));
    }
}
