#![allow(unused_variables)]
#![allow(incomplete_features)]
#![feature(impl_trait_in_bindings)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::{env, thread};

//use log::{info, warn, error};
//use crossbeam::thread as crossbeam_thread;
use grpcio::{ChannelBuilder, Environment, UnarySink};
use sharedlib::config::{REPLICAS_NUM, REPLICA_PORTS, SLOW_QUORUM};
use sharedlib::epaxos::{self as grpc_service, AcceptOKPayload, Empty, ReadResponse, WriteResponse};
use sharedlib::epaxos_grpc::{EpaxosService, EpaxosServiceClient};
use sharedlib::logic::{Accept, Commit, EpaxosLogic, Path, Payload, PreAccept,ReplicaId,WriteRequest};

#[derive(Clone)]
struct EpaxosServerImpl {
    inner: Arc<EpaxosServerInner>,
}

struct EpaxosServerInner {
    // In grpc, parameters in service are immutable.
    // See https://github.com/stepancheg/grpc-rust/blob/master/docs/FAQ.md
    store: Arc<Mutex<HashMap<String, i32>>>,
    epaxos_logic: Arc<Mutex<EpaxosLogic>>,
    replicas: HashMap<ReplicaId, EpaxosServiceClient>,
    quorum_members: Vec<ReplicaId>,
}

impl EpaxosServerImpl {
    pub fn init(id: ReplicaId, quorum_members: Vec<ReplicaId>) -> Self {
        Self {
            inner: Arc::new(EpaxosServerInner::init(id, quorum_members)),
        }
    }  
}

impl EpaxosServerInner {
    fn init(id: ReplicaId, quorum_members: Vec<ReplicaId>) -> Self {
        let mut replicas = HashMap::new();
        println!("Initializing Replica {}", id.0);
        for i in 0..REPLICAS_NUM {
            if i != id.0 as usize {
                let env = Arc::new(Environment::new(1));
                //let ch = ChannelBuilder::new(env).connect(format!("127.0.0.1:{}", REPLICA_PORTS[i]).as_str());
                //let addr = format!("127.0.0.1:{}", REPLICA_PORTS[i]).as_str();
                let mut addr = "127.0.0.1:".to_string();
                addr.push_str(REPLICA_PORTS[i]);
                //println!("{}", addr);
                //println!("{}", format!("127.0.0.1:{}", REPLICA_PORTS[i]));
                let ch = ChannelBuilder::new(env).connect(&addr);
                //println!(">> Neighbor replica {} created",i);
                let replica = EpaxosServiceClient::new(ch);
                replicas.insert(ReplicaId(i as u32), replica);
            }
        }

        EpaxosServerInner {
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
        println!("Send_Pre_accepts");
        let mut pre_accept_oks = Vec::new();
        for replica_id in self.quorum_members.iter() {
            //println!("Sending PreAccept to {:?}", replica_id);
            // crossbeam_thread::scope(|s| {
            //     s.spawn(|_| async {
            //         let pre_accept_ok = self.replicas
            //         .get(replica_id)
            //         .unwrap()
            //         .pre_accept_async(&payload.to_grpc());
            //         let value = pre_accept_ok.unwrap().await;
            //         match value {
            //             Err(e) => panic!("[PreAccept Stage] Replica panic"),
            //             Ok(_payload) => {
            //                 pre_accept_oks.push(Payload::from_grpc(&_payload));
            //             },
            //         }
            //     });
            // })
            // .unwrap();
            smol::block_on(async {
                let pre_accept_ok = self.replicas
                .get(replica_id)
                .unwrap()
                .pre_accept_async(&payload.to_grpc());
                let value = pre_accept_ok.unwrap().await;
                match value {
                    Err(e) => println!("[PreAccept Stage] Replica panic,{:?}", e),
                    Ok(_payload) => {
                        pre_accept_oks.push(Payload::from_grpc(&_payload));
                    },
                }
            });
        }
        pre_accept_oks
    }
    fn send_accepts(&self, payload: &Payload) -> usize {
        println!("Send_accept!!");
        let mut accept_ok_count: usize = 1;
        for replica_id in self.quorum_members.iter() {
            // crossbeam_thread::scope(|s| {
            //     s.spawn(|_| async{
            //         let accept_ok = self
            //             .replicas
            //             .get(replica_id)
            //             .unwrap()
            //             .accept_async(&payload.to_grpc());
            //         match accept_ok.unwrap().await {
            //             Err(e) => panic!("[Paxos-Accept Stage] Replica panic {:?}", e),
            //             Ok(_p) => {
            //                 accept_ok_count += 1;
            //             }
            //         }
            //     });
            // })
            // .unwrap();

            smol::block_on(async {
                let accept_ok = self
                    .replicas
                    .get(replica_id)
                    .unwrap()
                    .accept_async(&payload.to_grpc());
                    match accept_ok.unwrap().await {
                        Err(e) => panic!("[Paxos-Accept Stage] Replica panic {:?}", e),
                        Ok(_p) => {
                            accept_ok_count += 1;
                        }
                    }
            });
        }
        accept_ok_count
    }
    fn send_commits(&self, payload: &Payload) {
        println!("Send commits");
        for replica_id in self.quorum_members.iter() {
            // crossbeam_thread::scope(|s| {
            //     s.spawn(|_| async{
            //         let commit_ok = self.replicas.get(replica_id).unwrap().commit_async(&payload.to_grpc());
            //         //println!("Sending Commit to replica {}", replica_id.0);
            //         let value = commit_ok.unwrap().await;
            //         match value {
            //             Err(e) => panic!("[Commit Stage] Replica panic {:?}", e),
            //             Ok(_payload) => println!("Sending Commit to replica {}", replica_id.0),       
            //         }
            //     });
            // })
            // .unwrap();
            smol::block_on(async {
                let commit_ok = self.replicas.get(replica_id).unwrap().commit_async(&payload.to_grpc());
                //println!("Sending Commit to replica {}", replica_id.0);
                let value = commit_ok.unwrap().await;
                match value {
                    Err(e) => panic!("[Commit Stage] Replica panic {:?}", e),
                    Ok(_payload) => println!("Sending Commit to replica {}", replica_id.0),       
                }
            });
        }
    }

    fn execute(&self) {
        println!("Executing");
    }
}

impl EpaxosService for EpaxosServerImpl {
    fn write(&mut self, ctx: ::grpcio::RpcContext, req: sharedlib::epaxos::WriteRequest, sink: ::grpcio::UnarySink<WriteResponse>) {
        println!("write");
        let self_inner = Arc::<EpaxosServerInner>::clone(&self.inner);

        let task = async move {
            let mut r = grpc_service::WriteResponse::new();
            if self_inner.consensus(&WriteRequest::from_grpc(&req)) {
                (*self_inner.store.lock().unwrap()).insert(req.get_key().to_owned(), req.get_value());
                println!("DONE my store: {:#?}", self_inner.store.lock().unwrap());
                println!("Consensus successful. Sending a commit to client\n\n\n\n.");
                r.set_commit(true);

            }else {
                println!("Consensus failed. Notifying client.");
                r.set_commit(false);
            }
            Ok(r)
        };
        
        sharedlib::util::spawn_grpc_task(sink, task);
        //sink.success(r);
    }

    fn read(&mut self, ctx: ::grpcio::RpcContext, req: sharedlib::epaxos::ReadRequest, sink: ::grpcio::UnarySink<ReadResponse>) {
        println!("read");
        let self_inner = Arc::<EpaxosServerInner>::clone(&self.inner);
        let task = async move {
            println!("Received a read request with key = {}", req.get_key());
            self_inner.execute();
            let mut r = grpc_service::ReadResponse::new();
            //r.set_value(*((*self_inner.store.lock().unwrap()).get(req.get_key())).unwrap());
            let pp = &*self_inner.store.lock().unwrap();
            let ss = req.get_key();
            if ss.is_empty() {
                println!("Some thing wrong");
            }
            let rr = pp.get(ss);
            if rr.is_none() {
                r.set_value(i32::MAX);
            }else {
                r.set_value(*rr.unwrap());
            }
            //r.set_value(*rr.unwrap());
            // let oo = *rr;
            Ok(r)
        };
        sharedlib::util::spawn_grpc_task(sink, task);
        //sink.success(r);
    }

    fn pre_accept(&mut self, ctx: ::grpcio::RpcContext, req: sharedlib::epaxos::Payload, sink: UnarySink<sharedlib::epaxos::Payload>) {
        println!("Received PreAccept");
        let self_inner = Arc::<EpaxosServerInner>::clone(&self.inner);
        let task = async move {
            let mut epaxos_logic = self_inner.epaxos_logic.lock().unwrap();
            let request = PreAccept(Payload::from_grpc(&req));
            let r = epaxos_logic.pre_accept_(request);
            Ok(r.0.to_grpc())
        };
        sharedlib::util::spawn_grpc_task(sink, task);
        
        //sink.success(response.0.to_grpc());
    }

    fn accept(&mut self, ctx: ::grpcio::RpcContext, req: sharedlib::epaxos::Payload, sink: ::grpcio::UnarySink<AcceptOKPayload>) {
        println!("Accept");
        let self_inner = Arc::<EpaxosServerInner>::clone(&self.inner);
        let task = async move {
            let mut epaxos_logic = self_inner.epaxos_logic.lock().unwrap();
            let request = Accept(Payload::from_grpc(&req));
            let r = epaxos_logic.accept_(request);
            Ok(r.0.to_grpc())
        };
        sharedlib::util::spawn_grpc_task(sink, task);
        
        //sink.success(response.0.to_grpc());
    }

    fn commit(&mut self, ctx: ::grpcio::RpcContext, req: sharedlib::epaxos::Payload, sink: ::grpcio::UnarySink<Empty>) {
        println!("Commit");
        let self_inner = Arc::<EpaxosServerInner>::clone(&self.inner);
        let task = async move {
            let mut epaxos_logic = self_inner.epaxos_logic.lock().unwrap();
            let request = Commit(Payload::from_grpc(&req));
            epaxos_logic.commit_(request);
            let r = Empty::new();
            Ok(r)
        };
        sharedlib::util::spawn_grpc_task(sink, task);
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let id: u32 = args[1].parse().unwrap();
    let r1: u32 = args[2].parse().unwrap();
    let r2: u32 = args[3].parse().unwrap();

    // // let mut server_builder1 = grpc::ServerBuilder::new_plain();
    // // server_builder1.add_service(EpaxosServiceServer::new_service_def(EpaxosServer::init(
    // //     ReplicaId(1),
    // //     vec![ReplicaId(2), ReplicaId(3)],
    // // )));
    // let mut server = grpcio::ServerBuilder::new(env);
    // server_builder1.http.set_port(REPLICA_PORT);
    // let server1 = server_builder1.build().expect("build");
    // println!(">> Me {}", server1.local_addr());

    
    //let env = Arc::new(Environment::new(id as usize));
    //let ch = ChannelBuilder::new(env).connect(&(String::from(REPLICA_ADDRESSES[i as usize]) + &String::from(REPLICA_PORT)));
    let nn = EpaxosServerImpl::init(ReplicaId(id), vec![ReplicaId(r1), ReplicaId(r2)],);
    let service = sharedlib::epaxos_grpc::create_epaxos_service(nn);
    // let server_build = grpcio::ServerBuilder::new(env).register_service(service);
    // //.bind("127.0.0.1", 10000).build().unwrap();
    // let pre = server_build.bind("127.0.0.1", 10001).build().unwrap();
    

    let mut fake_server = grpcio::ServerBuilder::new(Arc::new(Environment::new(1)))
                                    .register_service(service)
                                    .bind("127.0.0.1", 10002)
                                    .build()
                                    .expect("Failed to build epaxos server");

    //pre.start();
    fake_server.start();
    println!("Server start!!!");
    // Blocks the main thread forever
    loop {
        thread::park();
    }
}
