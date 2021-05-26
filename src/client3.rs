use grpc::ClientStub;
use sharedlib::epaxos::*;
use sharedlib::epaxos_grpc::*;
use std::sync::Arc;
use futures::executor;

fn main() {
    //let grpc_client = Arc::new(grpc::Client::new_plain("127.0.0.1", 8080, Default::default()).unwrap());
    let grpc_client = grpc::ClientBuilder::new("127.0.0.1", 8080).build().unwrap();
    let client = EpaxosServiceClient::with_client(Arc::new(grpc_client));
    // let mut write_req = WriteRequest::new();
    // write_req.set_key("pi".to_owned());
    // write_req.set_value(3);
    // let write_resp = client.write(grpc::RequestOptions::new(), write_req);
    // println!("Client3 wrote {:?}", write_resp.wait());
    let mut read_req = ReadRequest::new();
    read_req.set_key("pi".to_owned());
    let read_resp = client.read(grpc::RequestOptions::new(), read_req);
    let res = read_resp.join_metadata_result();
    let r = executor::block_on(res);
    match r {
        Err(e) => panic!("Client3 panic {:?}", e),
        Ok((_, value, _)) => println!("Client3 read {:?}", value),
    }
}
