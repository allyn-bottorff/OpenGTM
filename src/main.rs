pub mod gtm;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::get,
    Router,
};

// use reqwest;
use serde::Deserialize;
// use serde_json;
use std::fs::File;
use std::io::BufReader;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::{collections::HashMap, error::Error};
use tokio::task::JoinSet;

#[derive(Deserialize)]
struct QueryParams {
    name: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // -----------------------------------------------------------------------
    // API SECTION
    // -----------------------------------------------------------------------

    println!("Starting health checkers");

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));

    let cache: Arc<Mutex<HashMap<String, Vec<gtm::Member>>>> = Arc::new(Mutex::new(HashMap::new()));

    let t = Arc::clone(&cache);
    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/livez", get(livez))
        .route("/info", get(info))
        .route("/reset", get(reset))
        .with_state(t);

    tokio::spawn(axum::Server::bind(&addr).serve(app.into_make_service()));

    // -----------------------------------------------------------------------
    // HEALTH CHECKER SECTION
    // -----------------------------------------------------------------------

    // let conf = vec![
    //     gtm::Pool {
    //         send: String::from("/healthy"),
    //         name: String::from("svc1"),
    //         port: 9090,
    //         members: vec!["localhost".into()],
    //         interval: 5,
    //         poll_type: gtm::PollType::HTTP,
    //     },
    //     gtm::Pool {
    //         send: String::from("/healthy"),
    //         name: String::from("svc2"),
    //         members: vec!["localhost".into()],
    //         port: 9090,
    //         interval: 15,
    //         poll_type: gtm::PollType::HTTP,
    //     },
    //     gtm::Pool {
    //         send: String::from("/unhealthy"),
    //         name: String::from("svc3"),
    //         members: vec!["localhost".into()],
    //         port: 9090,
    //         interval: 12,
    //         poll_type: gtm::PollType::HTTP,
    //     },
    // ];
    let conf = read_config(String::from("./conf.json")).unwrap();

    for c in &conf {
        let t = Arc::clone(&cache);
        let mut items = t.lock().unwrap();
        let mut members: Vec<gtm::Member> = c.members.iter().map(|m| gtm::Member::new(m)).collect();
        if let Some(fallback_ip) = c.fallback_ip {
            members.push(gtm::Member {
                host: "fallback".into(),
                ip: fallback_ip,
                healthy: true,
            });
        }

        items.insert(c.name.clone(), members);
    }

    // Run the "main" loop which calls other apis and updates the cache
    //
    // TODO: Reload config on some interrupt (like SIGHUP)
    //
    // Order of operations:
    // 1. Check list of mananged servers.
    // 2. Spawn a long-lived task for each checked server.
    // 3. Let each task loop over check interval.

    // Make an HTTP call to the local pingpong server. Return an error up to the tokio runtime
    // if something goes wrong.

    let mut join_set = JoinSet::new();

    for c in conf {
        for member in &c.members {
            let t = Arc::clone(&cache);
            let name = member.clone();
            join_set.spawn(c.clone().http_poller(name, t));
        }
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
async fn info(
    q: Query<QueryParams>,
    State(state): State<Arc<Mutex<HashMap<String, Vec<gtm::Member>>>>>,
) -> (StatusCode, String) {
    let map = &state.lock().unwrap();
    if let Some(item) = map.get(&q.name) {
        let healthy_members: Vec<&gtm::Member> =
            item.iter().filter(|m| m.healthy == true).collect();

        if let Some(member) = healthy_members.first() {
            (StatusCode::OK, member.ip.to_string())
        } else {
            (
                StatusCode::NOT_FOUND,
                "No healthy members and no fallback".into(),
            )
        }
    } else {
        (StatusCode::NOT_FOUND, "Not Found".into())
    }
}

/// Set the contents of the "localhost" entry of the host:ip map to be some
/// arbitrary IP to prove that the state is changing
async fn reset(
    q: Query<QueryParams>,
    State(state): State<Arc<Mutex<HashMap<String, Vec<gtm::Member>>>>>,
) -> (StatusCode, String) {
    state.lock().unwrap().insert(
        q.name.clone(),
        vec![gtm::Member {
            host: String::from("localhost"),
            ip: Into::into([1, 2, 3, 4]),
            healthy: true,
        }],
    );

    (StatusCode::OK, String::from("OK"))
}

/// Read config file
fn read_config(path: String) -> Result<Vec<gtm::Pool>, Box<dyn Error>> {
    // let mut conf: Vec<gtm::Pool> = Vec::new();
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let conf: Vec<gtm::Pool> = serde_json::from_reader(reader)?;

    Ok(conf)
}

#[cfg(test)]
mod tests {
    // use super::*;

    #[test]
    fn test_the_tests() {}
}
