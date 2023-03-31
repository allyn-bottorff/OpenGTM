use axum::http::StatusCode;
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
        let host_socket = format!("{}:{}", host, 443);

        // Get the first ipv4 address and ignore the rest
        // Explode if after filtering for an ipv4 addr, an ipv6 addr is parsed.
        let resolved_addr: Ipv4Addr = match &host_socket
            .to_socket_addrs()
            .unwrap()
            .filter(|ip| ip.is_ipv4())
            .next()
            .unwrap()
            .ip()
        {
            IpAddr::V4(ip) => *ip,
            IpAddr::V6(_) => panic!(
            "Found IPv6 after filtering out IPv6 addresses while trying to resolve hostname: {}",
            &host
        ), //This should be impossible.
        };

        Member {
            host: host.clone(),
            ip: resolved_addr,
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
        // TODO(alb): HTTP and HTTPS health checks
        // TODO(alb): TCP-only health checks
        // TODO(alb): Health checks which require authentication
        // TODO(alb): De-couple monitors and pools/pool members.

        let url = match self.poll_type {
            PollType::HTTP => format!("http://{}:{}{}", host, self.port, self.send),
            PollType::HTTPS => format!("https://{}:{}{}", host, self.port, self.send),
        };

        let host_socket = format!("{}:{}", host, self.port);


        loop {
            let client = match self.poll_type {
                PollType::HTTPS => reqwest::Client::builder()
                    .danger_accept_invalid_certs(true)
                    .build()
                    .unwrap(),
                PollType::HTTP =>  reqwest::Client::builder()
                    .build()
                    .unwrap(),
            };

            // Resolve the hostname once per iteration
            // TODO(alb): Handle this error more appropriately
            // This gets the first ipv4 addr and panics if it finds an ipv6
            let resolved_addr: Ipv4Addr =  match &host_socket
                .to_socket_addrs().unwrap()
                .filter(|ip| ip.is_ipv4())
                .next().expect("No IpV4 addresses found")
                .ip() {
                    IpAddr::V4(ip) =>  *ip,
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
