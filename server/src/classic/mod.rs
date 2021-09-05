#![feature(derive_default_enum)]
#![feature(destructuring_assignment)]

pub mod commit;
pub mod config;
pub mod converter;
pub mod epaxos;
pub mod epaxos_grpc;
pub mod execute;
pub mod server;
pub mod message;
pub mod util;
pub mod bloomfilter;

#[macro_use] 
extern crate lazy_static;
