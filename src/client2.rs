use grpcio::ChannelBuilder;

use grpcio::Environment;
use log::{error};
use sharedlib::epaxos::*;
use sharedlib::epaxos_grpc::*;
use std::sync::Arc;

fn main() {
    //let grpc_client = Arc::new(grpc::Client::new_plain(EU, REPLICA_PORT, Default::default()).unwrap());
    //let grpc_client = Arc::new(grpc::ClientBuilder::new("127.0.0.1",REPLICA_PORT,).build().unwrap());
    let env = Arc::new(Environment::new(1));
    let ch = ChannelBuilder::new(env).connect("127.0.0.1:10001");
    let client = EpaxosServiceClient::new(ch);
    //let client = EpaxosServiceClient::with_client(grpc_client);
    let mut write_req = WriteRequest::new();
    write_req.set_key("pi".to_owned());
    write_req.set_value(7);
    println!("start write");
    //let write_resp = client.write(&write_req);
    let resp = client.write(&write_req);
    println!("Client Just Write");
    //let wr_res = write_resp.unwrap();
    match resp {
        Err(_) => println!("hshsh"),
        Ok(_write_response) => {
            //let wr_res = write_resp.unwrap();
            println!("Write is success");
        },
    }
    let mut read_req = ReadRequest::new();
    read_req.set_key("pi".to_owned());
    //let read_resp = client.read(&read_req);
    let read_resp = client.read_async(&read_req);
    match read_resp {
        Err(e) => error!("Client2 panic {:?}", e),
        Ok(_read_response) => println!("Client2 read"),
    }
}
