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

use log::{error, info, warn};
use rand::prelude::*;
use reqwest;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
// use std::future::Pending;
use std::net::{IpAddr, Ipv4Addr, ToSocketAddrs};
use std::sync::{Arc, Mutex};
use tokio::{net, time};

#[derive(Clone, Deserialize)]
pub enum PollType {
    HTTP,
    TCP,
}

pub type HealthTable = Arc<Mutex<HashMap<String, Vec<Member>>>>;

#[derive(Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
enum HTTPReceive {
    StatusCodes(Vec<u16>),
    String(String),
}

#[derive(Clone, Serialize)]
pub struct Member {
    pub host: String,
    pub ip: Ipv4Addr,
    pub healthy: bool,
    pub cancel: bool,
}
impl PartialEq for Member {
    fn eq(&self, rhs: &Member) -> bool {
        self.host == rhs.host && self.ip == rhs.ip
    }
}
impl Member {
    pub fn new(host: &String) -> Member {
        let host_socket_string = format!("{}:{}", host, 443);

        // Get the first ipv4 address and ignore the rest
        // Explode if after filtering for an ipv4 addr, an ipv6 addr is parsed.
        // TODO(alb): this could probably be refactored to make it easier to read
        let resolved_addr: IpAddr = match host_socket_string.to_socket_addrs() {
            Ok(mut socket) => match socket.find(|ip| ip.is_ipv4()) {
                Some(socket_addr) => socket_addr.ip(),
                None => [127, 0, 0, 1].into(),
            },
            Err(_) => [127, 0, 0, 1].into(),
        };

        let resolved_v4 = match resolved_addr{
            IpAddr::V4(ip) => ip,
            IpAddr::V6(_) => panic!("Found IPv6 after filtering out IPv6 addresses while trying to resolve hostname: {}", &host),
        };

        Member {
            host: host.clone(),
            ip: resolved_v4,
            healthy: false,
            cancel: false,
        }
    }
}

#[derive(Clone, Deserialize)]
///Configuration relevant to the HTTP poll type
pub struct HTTPOptions {
    https_enabled: bool,
    https_require_validity: Option<bool>,
    send: String,
    receive_up: HTTPReceive,
}

#[derive(Clone, Deserialize)]
///Configuration relevant to a pool to be checked.
pub struct Pool {
    pub name: String, //FQDN label for load balanced app
    pub port: u16,
    pub interval: u16,
    pub members: Vec<String>, //Pool member FQDNs
    pub poll_type: PollType,
    pub http_options: Option<HTTPOptions>,
    pub fallback_ip: Option<Ipv4Addr>,
}

impl Pool {
    /// Long lived poller for TCP health checks.
    pub async fn tcp_poller(self, host: String, cache: HealthTable) {
        // Set backoff to a random integer value between 0 and the interval. At the end of the loop,
        // sleep the difference between the backoff and the configured interval. Ater the sleep, set
        // the interval to 0 so that the sleep is now the same as the interval.
        // This should keep the polling fairly even across the typical polling periods and prevent
        // blasting traffic out all at once on startup and then every 30 seconds after.

        info!("Starting poller for {}: {}", &self.name, &host);

        let backoff = rand::thread_rng().gen_range(0..=self.interval);
        let host_socket = format!("{}:{}", host, &self.port);

        info!(
            "Waiting {} seconds before starting poll for {}: {}",
            backoff, &self.name, &host
        );
        time::sleep(time::Duration::from_secs(backoff.into())).await;

        loop {
            // Resolve the hostname once per iteration
            // This gets the first ipv4 addr and panics if it finds an ipv6
            let mut socket = match host_socket.to_socket_addrs() {
                Ok(s) => s,
                Err(_) => {
                    warn!("DNS lookup failed for {}", &host);
                    time::sleep(time::Duration::from_secs(self.interval.into())).await;
                    continue;
                }
            };
            let resolved_addr: Ipv4Addr = match socket
                .find(|ip| ip.is_ipv4()).expect("No IpV4 addresses found")
                .ip() {
                    IpAddr::V4(ip) =>  ip,
                    IpAddr::V6(_) => panic!("Found IPv6 after filtering out IPv6 addresses while trying to resolve hostname: {}", &host) //This should be impossible.
                };
            let conn = net::TcpStream::connect(&host_socket).await;
            match conn {
                Ok(_) => set_health(&cache, &self.name, &host, &resolved_addr, true),
                Err(_) => set_health(&cache, &self.name, &host, &resolved_addr, false),
            }
            time::sleep(time::Duration::from_secs(self.interval.into())).await;
        }
    }

    /// Long lived poller for HTTP(s) health checks.
    pub async fn http_poller(self, host: String, cache: HealthTable) {
        // Set backoff to a random integer value between 0 and the interval. At the end of the loop,
        // sleep the difference between the backoff and the configured interval. Ater the sleep, set
        // the interval to 0 so that the sleep is now the same as the interval.
        // This should keep the polling fairly even across the typical polling periods and prevent
        // blasting traffic out all at once on startup and then every 30 seconds after.
        //
        // TODO(alb): Health checks which require authentication

        info!("Starting poller for {}: {}", &self.name, &host);

        let http_options = match &self.http_options {
            Some(o) => o,
            None => {
                error!(
                    "No http options found for http poller on pool {}. Exiting poller.",
                    &self.name
                );
                return;
            }
        };

        let url = match http_options.https_enabled {
            true => format!("https://{}:{}{}", host, self.port, http_options.send),
            false => format!("http://{}:{}{}", host, self.port, http_options.send),
        };

        let host_socket = format!("{}:{}", host, self.port);

        let backoff = rand::thread_rng().gen_range(0..=self.interval);

        info!(
            "Waiting {} seconds before starting poll for {}: {}",
            backoff, &self.name, &host
        );

        time::sleep(time::Duration::from_secs(backoff.into())).await;

        loop {
            let client = match &http_options.https_enabled {
                true => reqwest::Client::builder()
                    .danger_accept_invalid_certs(
                        !http_options.https_require_validity.unwrap_or(false),
                    )
                    .build()
                    .unwrap(),
                false => reqwest::Client::builder().build().unwrap(),
            };

            // Resolve the hostname once per iteration
            // This gets the first ipv4 addr and panics if it finds an ipv6
            let mut socket = match host_socket.to_socket_addrs() {
                Ok(s) => s,
                Err(_) => {
                    warn!("DNS lookup failed for {}", &host);
                    time::sleep(time::Duration::from_secs(self.interval.into())).await;
                    continue;
                }
            };
            let resolved_addr: Ipv4Addr = match socket
                .find(|ip| ip.is_ipv4()).expect("No IpV4 addresses found")
                .ip() {
                    IpAddr::V4(ip) =>  ip,
                    IpAddr::V6(_) => panic!("Found IPv6 after filtering out IPv6 addresses while trying to resolve hostname: {}", &host) //This should be impossible.
                };

            let req = client.get(&url).build().expect("Failed to build request.");

            info!("Checking health at {} for {}", &url, &self.name);

            // Check if the connection is successful
            // Mark the app healthy based on the kind of successs criteria defined on the pool
            match client.execute(req).await {
                Ok(r) => match &http_options.receive_up {
                    // Status code based healthy conditions
                    HTTPReceive::StatusCodes(codes) => {
                        if codes.contains(&r.status().as_u16()) {
                            set_health(&cache, &self.name, &host, &resolved_addr, true);
                        } else {
                            set_health(&cache, &self.name, &host, &resolved_addr, false);
                        }
                    }

                    // String matching based healthy conditions
                    HTTPReceive::String(match_string) => {
                        let r_bytes = match r.bytes().await {
                            Ok(b) => b,
                            Err(e) => {
                                set_health(&cache, &self.name, &host, &resolved_addr, false);
                                info!("{e}");
                                continue;
                            }
                        };

                        // Check if the received body contains the match string
                        if r_bytes
                            .windows(match_string.as_bytes().len())
                            .any(|window| window == match_string.as_bytes())
                        {
                            set_health(&cache, &self.name, &host, &resolved_addr, true);
                        } else {
                            set_health(&cache, &self.name, &host, &resolved_addr, false);
                        }
                    }
                },
                Err(_) => set_health(&cache, &self.name, &host, &resolved_addr, false),
            };
            if pending_cancel(&cache, &self.name, &host) {
                break;
            }

            time::sleep(time::Duration::from_secs(self.interval.into())).await;
        }
    }
}

/// Check for poller cancellation
fn pending_cancel(cache: &HealthTable, pool_name: &String, host: &String) -> bool {
    let mut pools = cache.lock().unwrap();
    if let Some(items) = pools.get_mut(pool_name) {
        for member in items.iter_mut() {
            if &member.host == host {
                if member.cancel {
                    member.cancel = false;
                    return true;
                } else {
                    return false;
                }
            }
        }
    }
    false
}

/// Set the health of the node in the sharead cache
fn set_health(
    cache: &HealthTable,
    pool_name: &String,
    host: &String,
    resolved_addr: &Ipv4Addr,
    health: bool,
) {
    match health {
        true => info!("Host: {} marked healthy for {}", &host, pool_name),
        false => info!("Host: {} marked unhealthy for {}", &host, pool_name),
    }
    let mut pools = cache.lock().unwrap();
    if let Some(items) = pools.get_mut(pool_name) {
        for member in items.iter_mut() {
            if &member.host == host {
                member.healthy = health;
                member.ip = *resolved_addr;
            }
        }
    }
}
