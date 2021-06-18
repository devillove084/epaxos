#![allow(unused_variables)]
#![allow(incomplete_features)]
#![feature(impl_trait_in_bindings)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;

use log::{info, trace, warn, error};
use crossbeam::thread as crossbeam_thread;
use grpcio::{ChannelBuilder, EnvBuilder, UnarySink};
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
        info!("Initializing Replica {}", id.0);
        for i in 0..REPLICAS_NUM {
            if i != id.0 as usize {
                let env = Arc::new(EnvBuilder::new().build());
                let ch = ChannelBuilder::new(env).connect("127.0.0.1:10000");
                info!(
                    ">> Neighbor replica {} created : {:?}",
                    i, REPLICA_ADDRESSES[i as usize]
                );
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
        info!("Starting consensus");
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
        info!("Send_Pre_accepts");
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
        info!("Send_accept!!");
        let mut accept_ok_count: usize = 1;
        for replica_id in self.quorum_members.iter() {
            crossbeam_thread::scope(|s| {
                s.spawn(|_| {
                    let accept_ok = self
                        .replicas
                        .get(replica_id)
                        .unwrap()
                        .accept(&payload.to_grpc());
                    match accept_ok {
                        Err(e) => error!("[Paxos-Accept Stage] Replica panic {:?}", e),
                        Ok(payload) => {
                            accept_ok_count += 1;
                        }
                    }
                });
            })
            .unwrap();
        }
        accept_ok_count
    }
    fn send_commits(&self, payload: &Payload) {
        info!("Send commits");
        for replica_id in self.quorum_members.iter() {
            crossbeam_thread::scope(|s| {
                s.spawn(|_| {
                    let res = self.replicas.get(replica_id).unwrap().commit(&payload.to_grpc());
                    info!("Sending Commit to replica {}", replica_id.0);
                });
            })
            .unwrap();
        }
    }

    fn execute(&self) {
        info!("Executing");
    }
}

impl EpaxosService for EpaxosServerImpl {
    fn write(&mut self, ctx: ::grpcio::RpcContext, req: sharedlib::epaxos::WriteRequest, sink: ::grpcio::UnarySink<WriteResponse>) {
        let self_inner = Arc::<EpaxosServerInner>::clone(&self.inner);

        let task = async move {
            let mut r = grpc_service::WriteResponse::new();
            if self_inner.consensus(&WriteRequest::from_grpc(&req)) {
                (*self_inner.store.lock().unwrap()).insert(req.get_key().to_owned(), req.get_value());
                info!("DONE my store: {:#?}", self_inner.store.lock().unwrap());
                info!("Consensus successful. Sending a commit to client\n\n\n\n.");
                r.set_commit(true);

            }else {
                warn!("Consensus failed. Notifying client.");
                r.set_commit(false);
            }
            Ok(r)
        };
        
        sharedlib::util::spawn_grpc_task(sink, task);
        //sink.success(r);
    }

    fn read(&mut self, ctx: ::grpcio::RpcContext, req: sharedlib::epaxos::ReadRequest, sink: ::grpcio::UnarySink<ReadResponse>) {
        let self_inner = Arc::<EpaxosServerInner>::clone(&self.inner);
        let task = async move {
            info!("Received a read request with key = {}", req.get_key());
            self_inner.execute();
            let mut r = grpc_service::ReadResponse::new();
            r.set_value(*((*self_inner.store.lock().unwrap()).get(req.get_key())).unwrap());
            Ok(r)
        };
        sharedlib::util::spawn_grpc_task(sink, task);
        //sink.success(r);
    }

    fn pre_accept(&mut self, ctx: ::grpcio::RpcContext, req: sharedlib::epaxos::Payload, sink: UnarySink<sharedlib::epaxos::Payload>) {
        info!("Received PreAccept");
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
        info!("Accept");
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
        info!("Commit");
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
    let nn = EpaxosServerImpl::init(ReplicaId(1), vec![ReplicaId(2), ReplicaId(3)],);
    let service = sharedlib::epaxos_grpc::create_epaxos_service(nn);
    let mut server_build = grpcio::ServerBuilder::new(env).register_service(service).bind("127.0.0.1", 10000).build().unwrap();
    server_build.start();
    loop {
        thread::park();
    }
}
