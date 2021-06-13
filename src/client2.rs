use grpcio::ChannelBuilder;
use grpcio::EnvBuilder;
use sharedlib::epaxos::*;
use sharedlib::epaxos_grpc::*;
use sharedlib::logic::{REPLICA_PORT};
use std::sync::Arc;

fn main() {
    //let grpc_client = Arc::new(grpc::Client::new_plain(EU, REPLICA_PORT, Default::default()).unwrap());
    //let grpc_client = Arc::new(grpc::ClientBuilder::new("127.0.0.1",REPLICA_PORT,).build().unwrap());
    let env = Arc::new(EnvBuilder::new().build());
    let ch = ChannelBuilder::new(env).connect(&(String::from("127.0.0.1")+ ":" + &String::from(REPLICA_PORT)));
    let client = EpaxosServiceClient::new(ch);
    //let client = EpaxosServiceClient::with_client(grpc_client);
    let mut write_req = WriteRequest::new();
    write_req.set_key("pi".to_owned());
    write_req.set_value(6);
    let write_resp = client.write(&write_req);
    let wr_res = write_resp.unwrap();
    // let wr_res = write_resp.join_metadata_result();
    // let wr_r = executor::block_on(wr_res);
    println!("Client2 wrote {:?}", wr_res);
    let mut read_req = ReadRequest::new();
    read_req.set_key("pi".to_owned());
    let read_resp = client.read(&read_req);
    // match read_resp.wait() {
    //     Err(e) => panic!("Client2 panic {:?}", e),
    //     Ok((_, value, _)) => println!("Client2 read {:?}", value),
    // }
    match read_resp {
        Err(e) => panic!("Client2 panic {:?}", e),
        Ok(_read_response) => println!("Client2 read"),
    }
}
