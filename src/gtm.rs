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
