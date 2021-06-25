#![allow(missing_docs)]

pub const SLOW_QUORUM: usize = 3; // F + 1
pub const FAST_QUORUM: usize = 3; // F + floor(F + 1 / 2)
pub const REPLICAS_NUM: usize = 5;
pub const LOCALHOST: &str = "127.0.0.1";

// // Local test
pub const REPLICA_PORT_1: &str = "10000";
pub const REPLICA_PORT_2: &str = "10001";
pub const REPLICA_PORT_3: &str = "10002";
pub const REPLICA_PORT_4: &str = "10003";
pub const REPLICA_PORT_5: &str = "10004";
// // Make host for test in different areas
pub static REPLICA_ADDRESSES: [&str; REPLICAS_NUM] = [LOCALHOST, LOCALHOST, LOCALHOST, LOCALHOST, LOCALHOST];
pub static REPLICA_PORTS: [&str; REPLICAS_NUM] = [REPLICA_PORT_1, REPLICA_PORT_2, REPLICA_PORT_3, REPLICA_PORT_4, REPLICA_PORT_5];