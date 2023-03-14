pub mod gtm;

use axum::{extract::{State, Query}, http::StatusCode, routing::get, Router};

use reqwest;
use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::task::JoinSet;
use serde::Deserialize;

#[derive(Clone)]
struct Config {
    send: String,
    name: String,
    host: String,
    port: u16,
    interval: u16,
    ip_addrs: Vec<Ipv4Addr>,
}

#[derive(Deserialize)]
struct QueryParams {
    name: String
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // API SECTION

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

    // HEALTH CHECKER SECTION

    let conf = vec![
        Config {
            send: String::from("/healthy"),
            name: String::from("svc1"),
            host: String::from("127.0.0.1"),
            port: 9090,
            interval: 5,
            ip_addrs: vec![Into::into([1, 1, 1, 1]), Into::into([1, 1, 1, 2])],
        },
        Config {
            send: String::from("/healthy"),
            name: String::from("svc2"),
            host: String::from("127.0.0.1"),
            port: 9090,
            interval: 15,
            ip_addrs: vec![Into::into([1, 1, 2, 1]), Into::into([1, 1, 2, 2])],
        },
        Config {
            send: String::from("/unhealthy"),
            name: String::from("svc3"),
            host: String::from("127.0.0.1"),
            port: 9090,
            interval: 12,
            ip_addrs: vec![Into::into([1, 1, 3, 1]), Into::into([1, 1, 3, 2])],
        },
    ];

    // Run the "main" loop which calls other apis and updates the cache
    // loop {
    // Order of operations:
    // 1. Check list of mananged servers.
    // 2. Spawn a task for each checked server.
    // 3. Let each task loop over check interval.

    // Make an HTTP call to the local pingpong server. Return an error up to the tokio runtime
    // if something goes wrong.

    let mut join_set = JoinSet::new();

    for c in &conf {
        let t = Arc::clone(&cache);
        join_set.spawn(health_poller(t, c.clone()));
    }

    while let Some(_res) = join_set.join_next().await {}
    Ok(())

    //     let resp = reqwest::get("http://127.0.0.1:9090/ping")
    //         .await?
    //         .text()
    //         .await?;
    //     println!("{}", resp);
    //
    //     // Grab a lock on the shared data and update the key with the address.
    //     let t = Arc::clone(&cache);
    //     {
    //         let mut ips = t.lock().unwrap();
    //         ips.insert(String::from("localhost"), Into::into([127, 0, 0, 1]));
    //     }
    //
    //     // Sleep to simulate long-ish running operations.
    //     tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    // }
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
async fn info(q: Query<QueryParams>, State(state): State<Arc<Mutex<HashMap<String, Ipv4Addr>>>>) -> (StatusCode, String) {

    // TODO(alb): handle the unwrap below better.
    let ip = state.lock().unwrap().get(&q.name).unwrap().to_string();

    (StatusCode::OK, ip)
}

/// Set the contents of the "localhost" entry of the host:ip map to be some
/// arbitrary IP to prove that the state is changing
async fn reset(q: Query<QueryParams>, State(state): State<Arc<Mutex<HashMap<String, Ipv4Addr>>>>) -> (StatusCode, String) {
    state
        .lock()
        .unwrap()
        .insert(q.name.clone(), Into::into([1, 2, 3, 4]));

    (StatusCode::OK, String::from("OK"))
}

/// Long lived task which can poll the target host the interval and set the result IP in the map.
async fn health_poller(cache: Arc<Mutex<HashMap<String, Ipv4Addr>>>, config: Config) {
    // Set backoff to random integer value between 0 and the interval. At the end of the loop,
    // sleep the difference between the backoff and the configured interval. Ater the sleep, set
    // the interval to 0 so that the sleep is now the same as the interval.
    // This should keep the polling fairly even across the typical polling periods and prevent
    // blasting traffic out all at once on startup and then every 30 seconds after.

    let url = format!("http://{}:{}{}", config.host, config.port, config.send);

    loop {
        match reqwest::get(&url).await {
            Ok(r) => {
                match r.status() {
                    StatusCode::OK => {
                        let mut ips = cache.lock().unwrap();
                        ips.insert(config.name.clone(), Into::into(config.ip_addrs[0]));
                    }
                    StatusCode::SERVICE_UNAVAILABLE => {
                        let mut ips = cache.lock().unwrap();
                        ips.insert(config.name.clone(), Into::into(config.ip_addrs[1]));
                    }
                    _ => {
                        let mut ips = cache.lock().unwrap();
                        ips.insert(config.name.clone(), Into::into(config.ip_addrs[1]));
                    }
                };
            }
            Err(_) => {
               let mut ips = cache.lock().unwrap();
                ips.insert(config.name.clone(), Into::into(config.ip_addrs[1]));
            }
        };

        tokio::time::sleep(tokio::time::Duration::from_secs(config.interval.into())).await;
    }
}

#[cfg(test)]
mod tests {
    // use super::*;

    #[test]
    fn test_the_tests() {}
}
