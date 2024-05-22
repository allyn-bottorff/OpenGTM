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
// use env_logger;
use log::{error, info};
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

#[derive(Deserialize)]
struct Config {
    pools: Vec<healthcheck::Pool>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // -----------------------------------------------------------------------
    // API SECTION
    // -----------------------------------------------------------------------

    env_logger::init();

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));

    let cache: healthcheck::HealthTable = Arc::new(Mutex::new(HashMap::new()));

    // TODO(alb): Separate into multiple IP info routes by type
    // e.g. "global availability", "round robin", "random"

    let t = Arc::clone(&cache);
    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/livez", get(livez))
        .route("/info", get(info))
        .route("/priority-order", get(handle_priority_order))
        .route("/randommember", get(info))
        .route("/reset", get(reset))
        .route("/dump", get(dump_table))
        .route("/reload", get(reload))
        .with_state(t);

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
    info!("Starting health checkers");
    loop {
        let conf = match read_config(String::from("./conf.json")) {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to parse config file");
                panic!("{e}");
            }
        };

        for p in &conf.pools {
            let mut members: Vec<healthcheck::Member> =
                p.members.iter().map(healthcheck::Member::new).collect();
            if let Some(fallback_ip) = p.fallback_ip {
                members.push(healthcheck::Member {
                    host: "fallback".into(),
                    ip: fallback_ip,
                    healthy: true,
                    cancel: false,
                });
            }
            let t = Arc::clone(&cache);
            let mut items = t.lock().unwrap();
            if !items.contains_key(&p.name) {
                items.insert(p.name.clone(), members);
            } else if let Some(pool) = items.get_mut(&p.name) {
                if *pool != members {
                    *pool = members;
                }
            }
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

        // Build out the table of health checks based on the loaded configuration.
        // Starts a poller referencing the same shared cache for each member
        for pool in conf.pools {
            for member in &pool.members {
                let t = Arc::clone(&cache);
                let name = member.clone();

                match pool.poll_type {
                    healthcheck::PollType::HTTP => {
                        join_set.spawn(pool.clone().http_poller(name, t))
                    }
                    healthcheck::PollType::TCP => join_set.spawn(pool.clone().tcp_poller(name, t)),
                };
            }
        }

        // while let Some(_res) = join_set.join_next().await {}
        if let Some(res) = join_set.join_next().await {
            match res {
                Ok(_) => {}
                Err(_) => {
                    break;
                }
            }
        }

        info!("Restarting health checkers");
    }
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
async fn info(
    q: Query<QueryParams>,
    State(state): State<healthcheck::HealthTable>,
) -> (StatusCode, String) {
    let map = &state.lock().unwrap();
    if let Some(item) = map.get(&q.name) {
        let healthy_members: Vec<&healthcheck::Member> =
            item.iter().filter(|m| m.healthy).collect();

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
    State(state): State<healthcheck::HealthTable>,
) -> (StatusCode, String) {
    let state = &state.lock();
    let map = match state {
        Ok(m) => m,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal Server Error".into(),
            )
        }
    };

    let members = match map.get(&q.name) {
        Some(p) => {
            let healthy_members: Vec<&healthcheck::Member> =
                p.iter().filter(|m| m.healthy).collect();
            healthy_members
        }
        None => return (StatusCode::NOT_FOUND, "Pool not found".into()),
    };

    if let Some(member) = members.first() {
        (StatusCode::OK, member.ip.to_string())
    } else {
        (
            StatusCode::NOT_FOUND,
            "No healthy members and no fallback IP".into(),
        )
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
    State(state): State<healthcheck::HealthTable>,
) -> (StatusCode, String) {
    state.lock().unwrap().insert(
        q.name.clone(),
        vec![healthcheck::Member {
            host: String::from("localhost"),
            ip: Into::into([1, 2, 3, 4]),
            healthy: true,
            cancel: false,
        }],
    );

    (StatusCode::OK, String::from("OK"))
}

/// Dump the entire state table to a JSON-formatted response
async fn dump_table(State(state): State<healthcheck::HealthTable>) -> (StatusCode, String) {
    let map = &state.lock().unwrap().clone();

    (StatusCode::OK, serde_json::to_string(map).unwrap())
}

/// Reload the config and restart the pollers
async fn reload(State(state): State<healthcheck::HealthTable>) -> (StatusCode, String) {
    let mut pools = state.lock().unwrap();
    for (_pool, members) in pools.iter_mut() {
        for member in members {
            member.cancel = true;
        }
    }

    (StatusCode::OK, String::from("Config reloaded"))
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
