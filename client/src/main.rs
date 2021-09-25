#![allow(unused_imports)]
#![allow(unused_variables)]

use grpcio::ChannelBuilder;
use grpcio::Environment;
use log::info;
use server::classic::epaxos::*;
use server::classic::epaxos_grpc::*;
use std::sync::Arc;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let addr = format!("{:?}", format_args!("{}:{}", &args[1], &args[2]));
    let env = Arc::new(Environment::new(1));
    let ch = ChannelBuilder::new(env).connect(&addr);
    let client = EpaxosServiceClient::new(ch);

    // let mut write_req = WriteRequest::new();
    // write_req.set_key("pi".to_owned());
    // write_req.set_value(7);
    // info!("start write");

    // smol::block_on(async {
    //     let resp = client.write_async(&write_req);
    //     let value = resp.unwrap().await;
    //     match value {
    //         Err(e) => panic!("Write no Responeses{}", e),
    //         Ok(_v) => {
    //             if _v.commit {
    //                 info!("Commit OK");
    //             }
    //         }
    //     }
    // });
    // info!("Client Just Write");

    // let mut read_req = ReadRequest::new();
    // read_req.set_key("pi".to_owned());

    // smol::block_on(async {
    //     let read_resp = client.read_async(&read_req);
    //     let value = read_resp.unwrap().await;
    //     match value {
    //         Err(e) => panic!("Write no Responeses{}", e),
    //         Ok(_v) => {
    //             info!("The final value is:{}", _v.value);
    //         }
    //     }
    // });
}
