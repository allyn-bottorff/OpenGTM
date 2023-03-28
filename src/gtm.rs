use axum::http::StatusCode;
use reqwest;
use std::collections::HashMap;
use std::net::Ipv4Addr;
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
#[derive(Clone)]
pub enum PollType {
    HTTP,
    HTTPS,
    TCP,
}

#[derive(Clone)]
pub struct Config {
    pub send: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub interval: u16,
    pub ip_addrs: Vec<Ipv4Addr>,
    pub poll_type: PollType,
}

impl Config {
    /// Long lived task which can poll the target host the interval and set the result IP in the map.
    pub async fn http_poller(self, cache: Arc<Mutex<HashMap<String, Ipv4Addr>>>) {
        // Set backoff to random integer value between 0 and the interval. At the end of the loop,
        // sleep the difference between the backoff and the configured interval. Ater the sleep, set
        // the interval to 0 so that the sleep is now the same as the interval.
        // This should keep the polling fairly even across the typical polling periods and prevent
        // blasting traffic out all at once on startup and then every 30 seconds after.
        //
        // TODO: HTTP and HTTPS health checks
        // TODO: TCP-only health checks
        // TODO: Health checks which require authentication
        // TODO: De-couple monitors and pools/pool members.

        let url = format!("http://{}:{}{}", self.host, self.port, self.send);

        loop {
            match reqwest::get(&url).await {
                Ok(r) => {
                    match r.status() {
                        StatusCode::OK => {
                            let mut ips = cache.lock().unwrap();
                            ips.insert(self.name.clone(), Into::into(self.ip_addrs[0]));
                        }
                        StatusCode::SERVICE_UNAVAILABLE => {
                            let mut ips = cache.lock().unwrap();
                            ips.insert(self.name.clone(), Into::into(self.ip_addrs[1]));
                        }
                        _ => {
                            let mut ips = cache.lock().unwrap();
                            ips.insert(self.name.clone(), Into::into(self.ip_addrs[1]));
                        }
                    };
                }
                Err(_) => {
                    let mut ips = cache.lock().unwrap();
                    ips.insert(self.name.clone(), Into::into(self.ip_addrs[1]));
                }
            };

            tokio::time::sleep(tokio::time::Duration::from_secs(self.interval.into())).await;
        }
    }

    pub async fn https_poller(self, cache: Arc<Mutex<HashMap<String, Ipv4Addr>>>) {
        // Set backoff to random integer value between 0 and the interval. At the end of the loop,
        // sleep the difference between the backoff and the configured interval. Ater the sleep, set
        // the interval to 0 so that the sleep is now the same as the interval.
        // This should keep the polling fairly even across the typical polling periods and prevent
        // blasting traffic out all at once on startup and then every 30 seconds after.
        //
        // TODO: HTTP and HTTPS health checks
        // TODO: TCP-only health checks
        // TODO: Health checks which require authentication
        // TODO: De-couple monitors and pools/pool members.

        let url = format!("https://{}:{}{}", self.host, self.port, self.send);

        loop {
            match reqwest::get(&url).await {
                Ok(r) => {
                    match r.status() {
                        StatusCode::OK => {
                            let mut ips = cache.lock().unwrap();
                            ips.insert(self.name.clone(), Into::into(self.ip_addrs[0]));
                        }
                        StatusCode::SERVICE_UNAVAILABLE => {
                            let mut ips = cache.lock().unwrap();
                            ips.insert(self.name.clone(), Into::into(self.ip_addrs[1]));
                        }
                        _ => {
                            let mut ips = cache.lock().unwrap();
                            ips.insert(self.name.clone(), Into::into(self.ip_addrs[1]));
                        }
                    };
                }
                Err(_) => {
                    let mut ips = cache.lock().unwrap();
                    ips.insert(self.name.clone(), Into::into(self.ip_addrs[1]));
                }
            };

            tokio::time::sleep(tokio::time::Duration::from_secs(self.interval.into())).await;
        }
    }
}
