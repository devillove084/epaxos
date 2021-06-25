#![allow(unused_variables)]

use grpcio::{ChannelBuilder, EnvBuilder};
//use log::info;
use rayon::prelude::*;
use sharedlib::epaxos_grpc::*;
use sharedlib::logic::{WriteRequest};
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
            let ch = ChannelBuilder::new(env).connect("127.0.0.1:10001");
            let client = EpaxosServiceClient::new(ch);
            let start = Instant::now();
            let r = client.write_async(req);
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
    println!("client 1 done!!");
}
