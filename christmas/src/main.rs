#[cfg(feature = "server")]
fn main() {
    use axum::routing::post;

    christmas::boot();

    // A custom router so one middleware can guard everything. Putting the check
    // in each server function instead would mean a new endpoint could ship
    // unprotected by omission.
    dioxus::server::serve(|| async {
        Ok(dioxus::server::router(christmas::App)
            .route("/auth/login", post(christmas::auth::login))
            .route("/auth/logout", post(christmas::auth::logout))
            .layer(axum::middleware::from_fn(christmas::auth::guard)))
    })
}

#[cfg(not(feature = "server"))]
fn main() {
    dioxus::launch(christmas::App);
}
