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

use axum::http::StatusCode;
use rand::prelude::*;
use reqwest;
use serde::Deserialize;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, ToSocketAddrs};
use std::sync::{Arc, Mutex};

// struct GTMApp {
//     name: String,
//     monitor: Monitor, //Monitor parameters to be applied to each member
//     pool: Vec<Member>, //members to be monitored
// }
//
// struct Monitor {
//     receive_up: String, // If this text exists anywhere in the response, the target is considered
//     // healthy. Does not take priority over `receiveDown`.
//     receive_down: String, // If this text exists anywhere in teh response, the target is considered
//     // unhealthy. Takes priority over `receiveUp`.
//     send: String, // HTTP send string for the health check.
// }
//
// struct Member {
//     hostname: String,
//     service_port: u16,
// }
#[derive(Clone, Deserialize)]
pub enum PollType {
    HTTP,
    HTTPS,
    // TCP, // TODO(alb): support basic TCP polling
}

#[derive(Clone)]
pub struct Member {
    pub host: String,
    pub ip: Ipv4Addr,
    pub healthy: bool,
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
            IpAddr::V6(_) => panic!("Found IPv6 after filtering out IPv6  addresses while trying to resolve hostname: {}", &host),
        };

        Member {
            host: host.clone(),
            ip: resolved_v4,
            healthy: false,
        }
    }
}

#[derive(Clone, Deserialize)]
pub struct Pool {
    pub send: String,
    pub name: String, //FQDN label for load balanced app
    pub port: u16,
    pub interval: u16,
    pub members: Vec<String>, //Pool member FQDNs
    pub poll_type: PollType,
    pub fallback_ip: Option<Ipv4Addr>,
}

impl Pool {
    /// Long lived task which can poll the target host the interval and set the result IP in the map.
    pub async fn http_poller(self, host: String, cache: Arc<Mutex<HashMap<String, Vec<Member>>>>) {
        // Set backoff to random integer value between 0 and the interval. At the end of the loop,
        // sleep the difference between the backoff and the configured interval. Ater the sleep, set
        // the interval to 0 so that the sleep is now the same as the interval.
        // This should keep the polling fairly even across the typical polling periods and prevent
        // blasting traffic out all at once on startup and then every 30 seconds after.
        //
        // TODO(alb): TCP-only health checks
        // TODO(alb): Health checks which require authentication
        // TODO(alb): De-couple monitors and pools/pool members.

        let url = match self.poll_type {
            PollType::HTTP => format!("http://{}:{}{}", host, self.port, self.send),
            PollType::HTTPS => format!("https://{}:{}{}", host, self.port, self.send),
        };

        let host_socket = format!("{}:{}", host, self.port);

        let backoff = rand::thread_rng().gen_range(0..=self.interval);

        tokio::time::sleep(tokio::time::Duration::from_secs(backoff.into())).await;

        loop {
            let client = match self.poll_type {
                PollType::HTTPS => reqwest::Client::builder()
                    .danger_accept_invalid_certs(true)
                    .build()
                    .unwrap(),
                PollType::HTTP => reqwest::Client::builder().build().unwrap(),
            };

            // Resolve the hostname once per iteration
            // This gets the first ipv4 addr and panics if it finds an ipv6
            let mut socket = match host_socket.to_socket_addrs() {
                Ok(s) => s,
                Err(_) => {
                    println!("DNS lookup failed for {}", &host);
                    tokio::time::sleep(tokio::time::Duration::from_secs(self.interval.into()))
                        .await;
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

            match client.execute(req).await {
                Ok(r) => {
                    match r.status() {
                        StatusCode::OK => {
                            let mut members = cache.lock().unwrap();
                            if let Some(items) = members.get_mut(&self.name) {
                                for member in items.iter_mut() {
                                    if member.host == host {
                                        member.healthy = true;
                                        member.ip = resolved_addr;
                                    }
                                }
                            } else {
                                continue;
                            }
                        }
                        StatusCode::SERVICE_UNAVAILABLE => {
                            let mut members = cache.lock().unwrap();
                            if let Some(items) = members.get_mut(&self.name) {
                                for member in items.iter_mut() {
                                    if member.host == host {
                                        member.healthy = false;
                                    }
                                }
                            } else {
                                continue;
                            }
                        }
                        _ => {
                            let mut members = cache.lock().unwrap();
                            if let Some(items) = members.get_mut(&self.name) {
                                for member in items.iter_mut() {
                                    if member.host == host {
                                        member.healthy = false;
                                    }
                                }
                            } else {
                                continue;
                            }
                        }
                    };
                }
                Err(_) => {
                    // let mut ips = cache.lock().unwrap();
                    // ips.insert(self.name.clone(), Into::into(self.ip_addrs[1]));
                }
            };

            tokio::time::sleep(tokio::time::Duration::from_secs(self.interval.into())).await;
        }
    }
}
