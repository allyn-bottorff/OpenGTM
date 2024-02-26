// Copyright 2023 Allyn L. Bottorff
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

pub mod healthcheck;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::get,
    Router,
};

// use reqwest;
use env_logger;
use log::{error, info};
use serde::Deserialize;
use serde_json;
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

#[derive(Deserialize)]
struct Config {
    pools: Vec<healthcheck::Pool>,
}

type HealthTable = HashMap<String, Arc<Mutex<Vec<healthcheck::Member>>>>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // -----------------------------------------------------------------------
    // API SECTION
    // -----------------------------------------------------------------------

    env_logger::init();

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));

    let state_table: HealthTable = HashMap::new();

    // TODO(alb): Separate into multiple IP info routes by type
    // e.g. "global availability", "round robin", "random"

    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/livez", get(livez))
        .route("/info", get(info))
        .route("/priority-order", get(handle_priority_order))
        .route("/randommember", get(info))
        .route("/reset", get(reset))
        .route("/dump", get(dump_table))
        .with_state(&state_table);

    info!("Starting API");

    tokio::spawn(
        axum::Server::bind(&addr)
            .tcp_nodelay(true)
            .serve(app.into_make_service()),
    );
    info!("API started");

    // -----------------------------------------------------------------------
    // HEALTH CHECKER SECTION
    // -----------------------------------------------------------------------

    let conf = match read_config(String::from("./conf.json")) {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to parse config file");
            panic!("{e}");
        }
    };

    for p in &conf.pools {
        let t = Arc::clone(&state_table);
        let mut items = t.lock().unwrap();
        let mut members: Vec<healthcheck::Member> = p
            .members
            .iter()
            .map(|m| healthcheck::Member::new(m))
            .collect();
        if let Some(fallback_ip) = p.fallback_ip {
            members.push(healthcheck::Member {
                host: "fallback".into(),
                ip: fallback_ip,
                healthy: true,
            });
        }

        items.insert(p.name.clone(), members);
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

    info!("Starting health checkers");

    for c in conf.pools {
        for member in &c.members {
            let t = Arc::clone(&state_table);
            let name = member.clone();

            match c.poll_type {
                healthcheck::PollType::HTTP => join_set.spawn(c.clone().http_poller(name, t)),
                healthcheck::PollType::TCP => join_set.spawn(c.clone().tcp_poller(name, t)),
            };
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

/// Get the first healthy member of the requested pool
async fn info(q: Query<QueryParams>, State(state): State<&HealthTable>) -> (StatusCode, String) {
    //let map = &state.lock().unwrap();
    if let Some(item) = state.get(&q.name) {
        let item = item.lock().unwrap();
        let healthy_members: Vec<&healthcheck::Member> =
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

/// Handler for the priority-order route. Returns the first healthy pool member or the fallback if
/// necessary
async fn handle_priority_order(
    q: Query<QueryParams>,
    State(state): State<&HealthTable>,
) -> (StatusCode, String) {
    if let Some(members) = state.get(&q.name) {
        let members = members.lock().unwrap();
        let healthy_members: Vec<&healthcheck::Member> =
            members.iter().filter(|m| m.healthy == true).collect();
        if let Some(healthy_member) = healthy_members.first() {
            (StatusCode::OK, healthy_member.ip.to_string())
        } else {
            (
                StatusCode::NOT_FOUND,
                "No healthy members found and no fallback IP".into(),
            )
        }
    } else {
        (StatusCode::NOT_FOUND, "Pool not found".into())
    }
}

//TODO(alb): finish the random order version (or round robin)

/// Handler for the random-member route. Returns a random selection from the healthy members
// async fn handle_random_order(
//     q: Query<QueryParams>,
//     State(state): State<Arc<Mutex<HashMap<String, Vec<healthcheck::Member>>>>>,
// ) -> (StatusCode, String) {
//     let state = &state.lock();
//
//     let map = match state {
//         Ok(m) => m,
//         Err(_) => {
//             return (
//                 StatusCode::INTERNAL_SERVER_ERROR,
//                 "
//
//         }
//     }
// }

/// Set the contents of the "localhost" entry of the host:ip map to be some
/// arbitrary IP to prove that the state is changing
async fn reset(
    q: Query<QueryParams>,
    State(state): State<&mut HealthTable>,
) -> (StatusCode, String) {
    state.insert(
        q.name.clone(),
        Arc::new(Mutex::new(vec![healthcheck::Member {
            host: String::from("localhost"),
            ip: Into::into([1, 2, 3, 4]),
            healthy: true,
        }])),
    );

    (StatusCode::OK, String::from("OK"))
}

/// Dump the entire state table to a JSON-formatted response. This creates a clone of the table
/// before serializing it and returning it to the client
async fn dump_table(State(state): State<&HealthTable>) -> (StatusCode, String) {
    let mut map = HashMap::new();
    for (key, val) in state {
        map.insert(key, val.lock().unwrap().clone());
    }
    (StatusCode::OK, serde_json::to_string(&map).unwrap())
}

/// Read config file
fn read_config(path: String) -> Result<Config, Box<dyn Error>> {
    // let mut conf: Vec<gtm::Pool> = Vec::new();
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let conf: Config = serde_json::from_reader(reader)?;

    Ok(conf)
}

#[cfg(test)]
mod tests {
    // use super::*;

    #[test]
    fn test_the_tests() {}
}
