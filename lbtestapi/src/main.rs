use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use std::{
    // fs,
    net::SocketAddr,
    sync::atomic::{AtomicBool, Ordering},
};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() {
    // let tls_cert: &str =
    // &fs::read_to_string("/etc/lbtestapi/tls.crt").expect("Failed to open TLS cert");
    // canonical location: /etc/lbtestapi/tls.crt
    // test location: ./tls.crt

    // let tls_key: &str =
    // &fs::read_to_string("/etc/lbtestapi/tls.key").expect("Failed to open TLS key");
    // canonical location: /etc/lbtestapi/tls.key
    // test location: ./tls.key
    //

    //let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));

    let health_status: Arc<AtomicBool> = Arc::new(AtomicBool::new(true));

    // TODO(alb): Separate into multiple IP info routes by type
    // e.g. "global availability", "round robin", "random"

    let t = Arc::clone(&health_status);
    let app = Router::new()
        .route("/health", get(healthz))
        .route("/short", get(short))
        .route("/long", get(long))
        .route("/instant", get(instant))
        .route("/togglehealth", post(toggle_health))
        .with_state(t);

    axum::Server::bind(&addr)
        .tcp_nodelay(true)
        .serve(app.into_make_service())
        .await
        .unwrap()
}

/// Service health probe
async fn healthz(State(state): State<Arc<AtomicBool>>) -> (StatusCode, &'static str) {
    let health_status = state.clone().load(Ordering::Relaxed);
    match health_status {
        true => (StatusCode::OK, "Healthy"),
        false => (StatusCode::SERVICE_UNAVAILABLE, "NOT OK"),
    }
}

/// Short Sleep
async fn short() -> (StatusCode, &'static str) {
    sleep(Duration::from_millis(20)).await;
    (StatusCode::OK, "OK")
}

/// Long sleep
async fn long() -> (StatusCode, &'static str) {
    sleep(Duration::from_millis(500)).await;
    (StatusCode::OK, "OK")
}

/// Instant response
async fn instant() -> (StatusCode, &'static str) {
    (StatusCode::OK, "OK")
}

/// Toggle Health status
async fn toggle_health(State(state): State<Arc<AtomicBool>>) -> (StatusCode, &'static str) {
    let health_status = state.clone().load(Ordering::Relaxed);

    match health_status {
        true => {
            state.store(false, Ordering::Relaxed);
            (StatusCode::OK, "Health status set to false.")
        }
        false => {
            state.store(true, Ordering::Relaxed);
            (StatusCode::OK, "Health status set to true.")
        }
    }
}
