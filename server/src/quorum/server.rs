#![allow(unused_variables)]
#![allow(incomplete_features)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

//use log::{info, warn, error};
use grpcio::{ChannelBuilder, Environment, UnarySink};
use super::config::{REPLICAS_NUM, REPLICA_TEST, SLOW_QUORUM};
use super::epaxos::{self as grpc_service, AcceptOKPayload, Empty, ReadResponse, WriteResponse};
use super::epaxos_grpc::{EpaxosService, EpaxosServiceClient};
use super::logic::{Accept, Commit, EpaxosLogic, Path, Payload, PreAccept,ReplicaId,WriteRequest};

#[derive(Clone)]
pub struct EpaxosServerImpl {
    pub inner: Arc<EpaxosServerInner>,
}

pub struct EpaxosServerInner {
    // In grpc, parameters in service are immutable.
    // See https://github.com/stepancheg/grpc-rust/blob/master/docs/FAQ.md
    pub store: Arc<Mutex<HashMap<String, i32>>>,
    pub epaxos_logic: Arc<Mutex<EpaxosLogic>>,
    pub replicas: HashMap<ReplicaId, EpaxosServiceClient>,
    pub quorum_members: Vec<ReplicaId>,
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
                let ch = ChannelBuilder::new(env).connect(REPLICA_TEST[i]);
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
            smol::block_on(async {
                let pre_accept_ok = self.replicas
                .get(replica_id)
                .unwrap()
                .pre_accept_async(&payload.to_grpc())
                .unwrap_or_else(|e| {
                    panic!("Panic on unwrap pre_accept,{}", e);
                });
                let value = pre_accept_ok.await;
                match value {
                    Err(e) => panic!("[PreAccept Stage] Replica panic,{:?}", e),
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
        let epaxos_logic = self.epaxos_logic.lock().unwrap();
        for replica_id in self.quorum_members.iter() {
            let cmd = &epaxos_logic.cmds;
            for map in cmd.iter() {
                for (slot, log) in map.iter() {
                    println!("The slot is{:?}", slot);
                    println!("+++++++");
                    println!("The log is{:?}", log);
                }
            }
        }
    }
}

impl EpaxosService for EpaxosServerImpl {
    fn write(&mut self, ctx: ::grpcio::RpcContext, req: super::epaxos::WriteRequest, sink: ::grpcio::UnarySink<WriteResponse>) {
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
        
        super::util::spawn_grpc_task(sink, task);

    }

    fn read(&mut self, ctx: ::grpcio::RpcContext, req: super::epaxos::ReadRequest, sink: ::grpcio::UnarySink<ReadResponse>) {
        println!("read");
        let self_inner = Arc::<EpaxosServerInner>::clone(&self.inner);
        let task = async move {
            println!("Received a read request with key = {}", req.get_key());
            println!("Start execute!!!");
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
            Ok(r)
        };
        super::util::spawn_grpc_task(sink, task);
    }

    fn pre_accept(&mut self, ctx: ::grpcio::RpcContext, req: super::epaxos::Payload, sink: UnarySink<super::epaxos::Payload>) {
        println!("Received PreAccept");
        let self_inner = Arc::<EpaxosServerInner>::clone(&self.inner);
        let task = async move {
            let mut epaxos_logic = self_inner.epaxos_logic.lock().unwrap();
            let request = PreAccept(Payload::from_grpc(&req));
            let r = epaxos_logic.pre_accept_(request);
            Ok(r.0.to_grpc())
        };
        super::util::spawn_grpc_task(sink, task);        
    }

    fn accept(&mut self, ctx: ::grpcio::RpcContext, req: super::epaxos::Payload, sink: ::grpcio::UnarySink<AcceptOKPayload>) {
        println!("Accept");
        let self_inner = Arc::<EpaxosServerInner>::clone(&self.inner);
        let task = async move {
            let mut epaxos_logic = self_inner.epaxos_logic.lock().unwrap();
            let request = Accept(Payload::from_grpc(&req));
            let r = epaxos_logic.accept_(request);
            Ok(r.0.to_grpc())
        };
        super::util::spawn_grpc_task(sink, task);
    }

    fn commit(&mut self, ctx: ::grpcio::RpcContext, req: super::epaxos::Payload, sink: ::grpcio::UnarySink<Empty>) {
        println!("Commit");
        let self_inner = Arc::<EpaxosServerInner>::clone(&self.inner);
        let task = async move {
            let mut epaxos_logic = self_inner.epaxos_logic.lock().unwrap();
            let request = Commit(Payload::from_grpc(&req));
            epaxos_logic.commit_(request);
            let r = Empty::new();
            Ok(r)
        };
        super::util::spawn_grpc_task(sink, task);
    }
}


