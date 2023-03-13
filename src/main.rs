pub mod gtm;

// use warp::Filter;

use axum::{extract::State, http::StatusCode, routing::get, Router};

use reqwest;
use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting health checkers");

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));

    let cache: Arc<Mutex<HashMap<String, Ipv4Addr>>> = Arc::new(Mutex::new(HashMap::new()));

    let t = Arc::clone(&cache);

    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/livez", get(livez))
        .route("/info", get(info))
        .route("/reset", get(reset))
        .with_state(t);

    tokio::spawn(axum::Server::bind(&addr).serve(app.into_make_service()));

    // let apps: Vec<GTMApp> = Vec::new();
    //
    // let pool: Vec<Member> = Vec::new();
    //
    // let p_member: Member = Member {
    //     hostname: String::from("127.0.0.1"),
    //     service_port:
    // }
    //
    // let app1: GTMApp = GTMApp {
    //     name: String::from("app1"),
    //     monitor: Monitor {
    //         receive_up: String::from("OK"),
    //         receive_down: String::from("503"),
    //         send: "/health",
    //     },
    //
    // };

    // Run the "main" loop which calls other apis and updates the cache
    loop {
        // Order of operations:
        // 1. Check list of mananged servers.
        // 2. Spawn a task for each checked server.
        // 3. Let each task loop over check interval.

        // Make an HTTP call to the local pingpong server. Return an error up to the tokio runtime
        // if something goes wrong.
        let resp = reqwest::get("http://127.0.0.1:9090/ping")
            .await?
            .text()
            .await?;
        println!("{}", resp);

        // Grab a lock on the shared data and update the key with the address.
        let t = Arc::clone(&cache);
        {
            let mut ips = t.lock().unwrap();
            ips.insert(String::from("localhost"), Into::into([127, 0, 0, 1]));
        }

        // Sleep to simulate long-ish running operations.
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    }
}

/// Service health probe
async fn healthz() -> (StatusCode, &'static str) {
    (StatusCode::OK, "OK")
}

/// Service liveness probe
async fn livez() -> (StatusCode, &'static str) {
    (StatusCode::OK, "OK")
}


/// Get the current value of the "localhost" entry of the host:ip Map
async fn info(State(state): State<Arc<Mutex<HashMap<String, Ipv4Addr>>>>) -> (StatusCode, String) {
    let ip = state.lock().unwrap()["localhost"].to_string();

    (StatusCode::OK, ip)
}


/// Set the contents of the "localhost" entry of the host:ip map to be some
/// arbitrary IP to prove that the state is changing
async fn reset(State(state): State<Arc<Mutex<HashMap<String, Ipv4Addr>>>>) -> (StatusCode, String) {
    state
        .lock()
        .unwrap()
        .insert(String::from("localhost"), Into::into([1, 2, 3, 4]));

    (StatusCode::OK, String::from("OK"))
}

#[cfg(test)]
mod tests {
    // use super::*;

    #[test]
    fn test_the_tests() {}
}
