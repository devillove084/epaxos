#![allow(unused_variables)]

use grpcio::{ChannelBuilder, EnvBuilder};
use rayon::prelude::*;
use sharedlib::epaxos_grpc::*;
use sharedlib::logic::{WriteRequest, REPLICA_ADDRESSES, REPLICA_PORT};
use std::{env, sync::Arc, time::Instant};

fn main() {
    let args: Vec<String> = env::args().collect();
    let id: u32 = args[1].parse().unwrap();
    let write_req1 = WriteRequest {
        key: "pi".to_string(),
        value: 1,
    };
    let mut write_reqs = Vec::new();
    write_reqs.push((write_req1.to_grpc(), id));
    write_reqs
        .par_iter_mut()
        .enumerate()
        .for_each(|(i, (req, id))| {
            let env = Arc::new(EnvBuilder::new().build());
            let ch = ChannelBuilder::new(env).connect(&(String::from(REPLICA_ADDRESSES[i as usize]) + &String::from(REPLICA_PORT)));
            // let grpc_client = Arc::new(
            //     // grpc::Client::new_plain(
            //     //     REPLICA_ADDRESSES[*id as usize],
            //     //     REPLICA_PORT,
            //     //     Default::default(),
            //     // )
            //     // .unwrap(),

            //     // grpc::ClientBuilder::new(REPLICA_ADDRESSES[*id as usize],
            //     //     REPLICA_PORT,).build().unwrap()
                
            // );
            let client = EpaxosServiceClient::new(ch);
            //let client = EpaxosServiceClient::with_client(grpc_client);
            let start = Instant::now();
            let r = client.write(req);
            // match res.wait() {
            //     Err(e) => panic!("Write Failed: {}", e),
            //     Ok((_, _, _)) => {
            //         let duration = start.elapsed();
            //         println!("{} Commit Latency: {:?}", i, duration);
            //     }
            // }
            match r {
                Err(e) => panic!("Write Failed: {}", e),
                Ok(_write_response) => {
                    let duration = start.elapsed();
                    println!("{} Commit Latency: {:?}", i, duration);
                }
            }
        });
}
