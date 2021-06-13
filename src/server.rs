#![allow(unused_variables)]
#![feature(impl_trait_in_bindings)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;

use crossbeam::thread as crossbeam_thread;
use grpcio::{ChannelBuilder, EnvBuilder, UnarySink};
use sharedlib::epaxos::{self as grpc_service, AcceptOKPayload, Empty, ReadResponse, WriteResponse};
use sharedlib::epaxos_grpc::{EpaxosService, EpaxosServiceClient};
use sharedlib::logic::{Accept, Commit, EpaxosLogic, Path, Payload, PreAccept, REPLICAS_NUM, REPLICA_ADDRESSES, REPLICA_PORT, ReplicaId, SLOW_QUORUM, WriteRequest};

#[derive(Clone)]
struct EpaxosServer {
    // In grpc, parameters in service are immutable.
    // See https://github.com/stepancheg/grpc-rust/blob/master/docs/FAQ.md
    store: Arc<Mutex<HashMap<String, i32>>>,
    epaxos_logic: Arc<Mutex<EpaxosLogic>>,
    replicas: HashMap<ReplicaId, EpaxosServiceClient>,
    quorum_members: Vec<ReplicaId>,
}

impl EpaxosServer {
    fn init(id: ReplicaId, quorum_members: Vec<ReplicaId>) -> EpaxosServer {
        let mut replicas = HashMap::new();
        println!("Initializing Replica {}", id.0);
        for i in 0..REPLICAS_NUM {
            if i != id.0 as usize {
                // let internal_client = grpc::Client::new_plain(
                //     REPLICA_ADDRESSES[i as usize],
                //     REPLICA_PORT,
                //     Default::default(),
                // ).unwrap();
                //let internal_client = grpc::ClientBuilder::new(REPLICA_ADDRESSES[i as usize], REPLICA_PORT).build().unwrap();
                let env = Arc::new(EnvBuilder::new().build());
                //let ch = ChannelBuilder::new(env).connect(CONTROLLER_END_POINT);
                let ch = ChannelBuilder::new(env).connect(&(String::from(REPLICA_ADDRESSES[i as usize]) + &String::from(REPLICA_PORT)));
                println!(
                    ">> Neighbor replica {} created : {:?}",
                    i, REPLICA_ADDRESSES[i as usize]
                );
                //let replica = EpaxosServiceClient::with_client(Arc::new(internal_client));
                let replica = EpaxosServiceClient::new(ch);
                replicas.insert(ReplicaId(i as u32), replica);
            }
        }

        EpaxosServer {
            store: Arc::new(Mutex::new(HashMap::new())),
            epaxos_logic: Arc::new(Mutex::new(EpaxosLogic::init(id))),
            replicas: replicas,
            quorum_members: quorum_members,
        }
    }

    // we only need to do consensus for write req
    fn consensus(&self, write_req: &WriteRequest) -> bool {
        println!("Starting consensus");
        let mut epaxos_logic = self.epaxos_logic.lock().unwrap();
        let payload = epaxos_logic.lead_consensus(write_req.clone());
        let pre_accept_oks = self.send_pre_accepts(&payload);

        match epaxos_logic.decide_path(pre_accept_oks, &payload) {
            Path::Fast(payload_) => {
                // Send Commit message to F
                self.send_commits(&payload_);
                epaxos_logic.committed(payload_);
                return true;
            }
            Path::Slow(payload_) => {
                // Start Paxos-Accept stage
                // Send Accept message to F
                epaxos_logic.accepted(payload_.clone());
                if self.send_accepts(&payload_) >= SLOW_QUORUM {
                    self.send_commits(&payload_);
                    epaxos_logic.committed(payload_);
                    return true;
                }
                return false;
            }
        }
    }

    fn send_pre_accepts(&self, payload: &Payload) -> Vec<Payload> {
        let mut pre_accept_oks = Vec::new();
        for replica_id in self.quorum_members.iter() {
            //println!("Sending PreAccept to {:?}", replica_id);
            crossbeam_thread::scope(|s| {
                s.spawn(|_| {
                    //let pre_accept_ok = self.replicas.get(replica_id).unwrap().pre_accept(grpc::RequestOptions::new(), payload.to_grpc());
                    let pre_accept_ok = self.replicas
                    .get(replica_id)
                    .unwrap()
                    .pre_accept(&payload.to_grpc());
                    // match pre_accept_ok.wait() {
                    //     Err(e) => panic!("[PreAccept Stage] Replica panic {:?}", e),
                    //     Ok((_, value, _)) => {
                    //         pre_accept_oks.push(Payload::from_grpc(&value));
                    //     }
                    // }
                    let p = pre_accept_ok.unwrap();
                    pre_accept_oks.push(Payload::from_grpc(&p));
                    // let pre = pre_accept_ok;
                    // match pre {
                    //     Err(_) => panic!("[PreAccept Stage] Replica panic"),
                    //     Ok(_) => {
                    //         pre_accept_oks.push(Payload::from_grpc(&p));
                    //     }
                    // }
                    // let res = pre_accept_ok.join_metadata_result();
                    // let r = executor::block_on(res);
                    // match r {
                    //     Err(e) => panic!("[PreAccept Stage] Replica panic {:?}", e),
                    //     Ok((_, value, _)) => {
                    //         pre_accept_oks.push(Payload::from_grpc(&value));
                    //     }
                    // }
                });
            })
            .unwrap();
        }
        pre_accept_oks
    }
    fn send_accepts(&self, payload: &Payload) -> usize {
        let mut accept_ok_count: usize = 1;
        for replica_id in self.quorum_members.iter() {
            crossbeam_thread::scope(|s| {
                s.spawn(|_| {
                    let accept_ok = self
                        .replicas
                        .get(replica_id)
                        .unwrap()
                        .accept(&payload.to_grpc());
                    // match accept_ok.wait() {
                    //     Err(e) => panic!("[Paxos-Accept Stage] Replica panic {:?}", e),
                    //     Ok((_, _, _)) => {
                    //         accept_ok_count += 1;
                    //     }
                    // }
                    match accept_ok {
                        Err(e) => panic!("[Paxos-Accept Stage] Replica panic {:?}", e),
                        Ok(Payload) => {
                            accept_ok_count += 1;
                        }
                    }
                    // let res = accept_ok.join_metadata_result();
                    // let r = executor::block_on(res);
                    // match r {
                    //     Err(e) => panic!("[Paxos-Accept Stage] Replica panic {:?}", e),
                    //     Ok((_, _, _)) => {
                    //         accept_ok_count += 1;
                    //     }
                    // }
                });
            })
            .unwrap();
        }
        accept_ok_count
    }
    fn send_commits(&self, payload: &Payload) {
        for replica_id in self.quorum_members.iter() {
            crossbeam_thread::scope(|s| {
                s.spawn(|_| {
                    self.replicas
                        .get(replica_id)
                        .unwrap()
                        .commit(&payload.to_grpc());
                    println!("Sending Commit to replica {}", replica_id.0);
                });
            })
            .unwrap();
        }
    }

    fn execute(&self) {
        println!("Executing");
    }
}

impl EpaxosService for EpaxosServer {
    // fn write(
    //     &self,
    //     _m: grpc::RequestOptions,
    //     req: grpc_service::WriteRequest,
    // ) -> grpc::SingleResponse<grpc_service::WriteResponse> {
    //     println!(
    //         "Received a write request with key = {} and value = {}",
    //         req.get_key(),
    //         req.get_value()
    //     );
    //     let mut r = grpc_service::WriteResponse::new();
    //     if self.consensus(&WriteRequest::from_grpc(&req)) {
    //         // TODO when do I actually execute?
    //         (*self.store.lock().unwrap()).insert(req.get_key().to_owned(), req.get_value());
    //         println!("DONE my store: {:#?}", self.store.lock().unwrap());
    //         println!("Consensus successful. Sending a commit to client\n\n\n\n.");
    //         r.set_commit(true);
    //     } else {
    //         println!("Consensus failed. Notifying client.");
    //         r.set_commit(false);
    //     }
    //     grpc::SingleResponse::completed(r)
    // }

    // fn write(
    //     &self,
    //     o: grpc::ServerHandlerContext,
    //     req: grpc::ServerRequestSingle<sharedlib::epaxos::WriteRequest>,
    //     resp: grpc::ServerResponseUnarySink<sharedlib::epaxos::WriteResponse>) -> ::grpc::Result<()> {
    //     let req_new = req.message;
    //     let mut r = grpc_service::WriteResponse::new();
    //     if self.consensus(&WriteRequest::from_grpc(&req_new)) {
    //         (*self.store.lock().unwrap()).insert(req_new.get_key().to_owned(), req_new.get_value());
    //         println!("DONE my store: {:#?}", self.store.lock().unwrap());
    //         println!("Consensus successful. Sending a commit to client\n\n\n\n.");
    //         r.set_commit(true);

    //     }else {
    //         println!("Consensus failed. Notifying client.");
    //         r.set_commit(false);
    //     }
    //     let result = grpc::SingleResponse::completed(r);
    //     Ok(())
    // }

    fn write(&mut self, ctx: ::grpcio::RpcContext, req: sharedlib::epaxos::WriteRequest, sink: ::grpcio::UnarySink<WriteResponse>) {
        let mut r = grpc_service::WriteResponse::new();
        if self.consensus(&WriteRequest::from_grpc(&req)) {
            (*self.store.lock().unwrap()).insert(req.get_key().to_owned(), req.get_value());
            println!("DONE my store: {:#?}", self.store.lock().unwrap());
            println!("Consensus successful. Sending a commit to client\n\n\n\n.");
            r.set_commit(true);

        }else {
            println!("Consensus failed. Notifying client.");
            r.set_commit(false);
        }
        sink.success(r);
    }

    // fn read(
    //     &self,
    //     _m: grpc::RequestOptions,
    //     req: grpc_service::ReadRequest,
    // ) -> grpc::SingleResponse<grpc_service::ReadResponse> {
    //     println!("Received a read request with key = {}", req.get_key());
    //     self.execute();
    //     let mut r = grpc_service::ReadResponse::new();
    //     r.set_value(*((*self.store.lock().unwrap()).get(req.get_key())).unwrap());
    //     grpc::SingleResponse::completed(r)
    // }
    // fn read(
    //     &self, 
    //     o: ::grpc::ServerHandlerContext, 
    //     req: ::grpc::ServerRequestSingle<ReadRequest>, 
    //     resp: ::grpc::ServerResponseUnarySink<ReadResponse>) -> grpc::Result<()> {
    //         let r_req = req.message;

    //         println!("Received a read request with key = {}", r_req.get_key());
    //         self.execute();
    //         let mut r = grpc_service::ReadResponse::new();
    //         r.set_value(*((*self.store.lock().unwrap()).get(r_req.get_key())).unwrap());
    //         grpc::SingleResponse::completed(r);
    //         Ok(())
    // }
    fn read(&mut self, ctx: ::grpcio::RpcContext, req: sharedlib::epaxos::ReadRequest, sink: ::grpcio::UnarySink<ReadResponse>) {
            println!("Received a read request with key = {}", req.get_key());
            self.execute();
            let mut r = grpc_service::ReadResponse::new();
            r.set_value(*((*self.store.lock().unwrap()).get(req.get_key())).unwrap());
            sink.success(r);
    }

    // fn pre_accept(
    //     &self,
    //     _o: grpc::RequestOptions,
    //     p: grpc_service::Payload,
    // ) -> grpc::SingleResponse<grpc_service::Payload> {
    //     println!("Received PreAccept");
    //     let mut epaxos_logic = self.epaxos_logic.lock().unwrap();
    //     let request = PreAccept(Payload::from_grpc(&p));
    //     let response = epaxos_logic.pre_accept_(request);
    //     grpc::SingleResponse::completed(response.0.to_grpc())
    //}
    // fn pre_accept(
    //     &self, 
    //     o: grpc::ServerHandlerContext, 
    //     req: grpc::ServerRequestSingle<sharedlib::epaxos::Payload>, 
    //     resp: grpc::ServerResponseUnarySink<sharedlib::epaxos::Payload>) -> ::grpc::Result<()> {
        
    //         println!("Received PreAccept");
    //         let r = req.message;
    //         let mut epaxos_logic = self.epaxos_logic.lock().unwrap();
    //         let request = PreAccept(Payload::from_grpc(&r));
    //         let response = epaxos_logic.pre_accept_(request);
    //         grpc::SingleResponse::completed(response.0.to_grpc());
    //         Ok(())    
    // }

    fn pre_accept(&mut self, ctx: ::grpcio::RpcContext, req: sharedlib::epaxos::Payload, sink: UnarySink<sharedlib::epaxos::Payload>) {
        println!("Received PreAccept");
        let mut epaxos_logic = self.epaxos_logic.lock().unwrap();
        let request = PreAccept(Payload::from_grpc(&req));
        let response = epaxos_logic.pre_accept_(request);
        sink.success(response.0.to_grpc());
    }

    // fn accept(
    //     &self,
    //     _o: grpc::RequestOptions,
    //     p: grpc_service::Payload,
    // ) -> grpc::SingleResponse<grpc_service::AcceptOKPayload> {
    //     let mut epaxos_logic = self.epaxos_logic.lock().unwrap();
    //     let request = Accept(Payload::from_grpc(&p));
    //     let response = epaxos_logic.accept_(request);
    //     grpc::SingleResponse::completed(response.0.to_grpc())
    // }

    // fn accept(
    //     &self, 
    //     o: ::grpc::ServerHandlerContext, 
    //     req: ::grpc::ServerRequestSingle<sharedlib::epaxos::Payload>, 
    //     resp: ::grpc::ServerResponseUnarySink<sharedlib::epaxos::AcceptOKPayload>) -> ::grpc::Result<()> {
        
    //         let r = req.message;
    //         let mut epaxos_logic = self.epaxos_logic.lock().unwrap();
    //         let request = Accept(Payload::from_grpc(&r));
    //         let response = epaxos_logic.accept_(request);
    //         grpc::SingleResponse::completed(response.0.to_grpc());
    //         Ok(())

    // }

    fn accept(&mut self, ctx: ::grpcio::RpcContext, req: sharedlib::epaxos::Payload, sink: ::grpcio::UnarySink<AcceptOKPayload>) {
        let mut epaxos_logic = self.epaxos_logic.lock().unwrap();
        let request = Accept(Payload::from_grpc(&req));
        let response = epaxos_logic.accept_(request);
        sink.success(response.0.to_grpc());
    }

    // fn commit(
    //     &self,
    //     _o: grpc::RequestOptions,
    //     p: grpc_service::Payload,
    // ) -> grpc::SingleResponse<grpc_service::Empty> {
    //     let mut epaxos_logic = self.epaxos_logic.lock().unwrap();
    //     let request = Commit(Payload::from_grpc(&p));
    //     epaxos_logic.commit_(request);
    //     grpc::SingleResponse::completed(grpc_service::Empty::new())
    // }

    // fn commit(
    //     &self, 
    //     o: grpc::ServerHandlerContext, 
    //     req: grpc::ServerRequestSingle<sharedlib::epaxos::Payload>, 
    //     resp: grpc::ServerResponseUnarySink<sharedlib::epaxos::Empty>) -> grpc::Result<()> {
            
    //         let r = req.message;
    //         let mut epaxos_logic = self.epaxos_logic.lock().unwrap();
    //         let request = Commit(Payload::from_grpc(&r));
    //         epaxos_logic.commit_(request);
    //         grpc::SingleResponse::completed(grpc_service::Empty::new());
    //         Ok(())
    // }

    fn commit(&mut self, ctx: ::grpcio::RpcContext, req: sharedlib::epaxos::Payload, sink: ::grpcio::UnarySink<Empty>) {
        let mut epaxos_logic = self.epaxos_logic.lock().unwrap();
        let request = Commit(Payload::from_grpc(&req));
        epaxos_logic.commit_(request);
    }
}

fn main() {
    // let args: Vec<String> = env::args().collect();

    // // let id: u32 = args[1].parse().unwrap();
    // // let r1: u32 = args[2].parse().unwrap();
    // // let r2: u32 = args[3].parse().unwrap();
    // // let mut server_builder1 = grpc::ServerBuilder::new_plain();
    // // server_builder1.add_service(EpaxosServiceServer::new_service_def(EpaxosServer::init(
    // //     ReplicaId(1),
    // //     vec![ReplicaId(2), ReplicaId(3)],
    // // )));
    // let mut server = grpcio::ServerBuilder::new(env);
    // server_builder1.http.set_port(REPLICA_PORT);
    // let server1 = server_builder1.build().expect("build");
    // println!(">> Me {}", server1.local_addr());

    // Blocks the main thread forever
    let env = Arc::new(EnvBuilder::new().build());
    //let ch = ChannelBuilder::new(env).connect(&(String::from(REPLICA_ADDRESSES[i as usize]) + &String::from(REPLICA_PORT)));
    let nn = EpaxosServer::init(ReplicaId(1), vec![ReplicaId(2), ReplicaId(3)],);
    let service = sharedlib::epaxos_grpc::create_epaxos_service(nn);
    let mut server_build = grpcio::ServerBuilder::new(env).register_service(service).bind("127.0.0.1", 10000).build().unwrap();
    server_build.start();
    loop {
        thread::park();
    }
}
