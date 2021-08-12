#![allow(unused_variables)]
#![allow(incomplete_features)]

use std::cmp::max;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::commit::{Prepare, PrepareOKPayload, PreparePayload, PrepareStage};

use super::config::{REPLICAS_NUM, REPLICA_TEST, SLOW_QUORUM};
use super::epaxos::{self as grpc_service, AcceptOKPayload, Empty, ReadResponse, WriteResponse};
use super::epaxos_grpc::{EpaxosService, EpaxosServiceClient};
use super::commit::{
    Accept, Commit, EpaxosLogic, Path, Payload, PreAccept, ReplicaId, WriteRequest,
};
use grpcio::{ChannelBuilder, Environment, UnarySink};
use log::{error, info};

#[derive(Clone)]
pub struct EpaxosServerImpl {
    pub inner: Arc<EpaxosServerInner>,
}

pub struct EpaxosServerInner {
    pub store: Arc<Mutex<HashMap<String, i32>>>,
    pub epaxos_logic: Arc<Mutex<EpaxosLogic>>,
    pub replicas: HashMap<ReplicaId, EpaxosServiceClient>,
    pub quorum_members: Vec<ReplicaId>,
    pub max_ballot: u32,
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
                let env = Arc::new(Environment::new(1));
                let ch = ChannelBuilder::new(env).connect(REPLICA_TEST[i]);
                info!(">> Neighbor replica {} created", i);
                let replica = EpaxosServiceClient::new(ch);
                replicas.insert(ReplicaId(i as u32), replica);
            }
        }

        EpaxosServerInner {
            store: Arc::new(Mutex::new(HashMap::new())),
            epaxos_logic: Arc::new(Mutex::new(EpaxosLogic::init(id))),
            replicas: replicas,
            quorum_members: quorum_members,
            max_ballot: 0,
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

    fn explicit_prepare(&self, payload: &Payload) {
        let mut epaxos_logic = self.epaxos_logic.lock().unwrap();
        // Paper: where epoch.b.R is the highest ballot number Q is aware of in instance L.i
        let mut pre_payload = epaxos_logic.aware_ballot(payload);
        let mut replies = self.send_prepare(&pre_payload);
        
        match epaxos_logic.decide_prepare(replies, payload) {
            PrepareStage::Commit(_payload) => {
                self.send_commits(&_payload);
                epaxos_logic.committed(_payload);
            },
            PrepareStage::PaxosAccept(_payload) => {
                if self.send_accepts(&_payload) >= SLOW_QUORUM {
                    self.send_commits(&_payload);
                    epaxos_logic.committed(_payload);
                }
            },
            PrepareStage::PhaseOne(_payload) => {
                //TODO: start Phase 1 (at line 2) for γ at L.i, 
                // start Phase 1 (at line 2) for no-op at L.i
            },
        }

    }

    fn send_pre_accepts(&self, payload: &Payload) -> Vec<Payload> {
        info!("Send_Pre_accepts");
        let mut pre_accept_oks = Vec::new();
        for replica_id in self.quorum_members.iter() {
            info!("Sending PreAccept to {:?}", replica_id);
            smol::block_on(async {
                let pre_accept_ok = self
                    .replicas
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
                    }
                }
            });
        }
        pre_accept_oks
    }

    fn send_accepts(&self, payload: &Payload) -> usize {
        info!("Send_accept!!");
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
        info!("Send commits");
        for replica_id in self.quorum_members.iter() {
            smol::block_on(async {
                let commit_ok = self
                    .replicas
                    .get(replica_id)
                    .unwrap()
                    .commit_async(&payload.to_grpc());
                info!("Sending Commit to replica {}", replica_id.0);
                let value = commit_ok.unwrap().await;
                match value {
                    Err(e) => error!("[Commit Stage] Replica panic {:?}", e),
                    Ok(_payload) => info!("Sending Commit to replica {}", replica_id.0),
                }
            });
        }
    }

    fn send_prepare(&self, payload: &Payload) -> Vec<PrepareOKPayload>{
        info!("Send Explicit Prepare");
        let mut reply_set = Vec::new();
        let mut epaxos_logic = self.epaxos_logic.lock().unwrap();
        // highest ballot number
        for replica_id in self.quorum_members.iter() {
            smol::block_on(async {
                let prepare_ok = self.replicas.get(replica_id).unwrap().prepare_async(&payload.to_grpc());
                let value = prepare_ok.unwrap().await;
                match value {
                    Ok(_payload) => {
                        if _payload.ballot == self.max_ballot {
                            reply_set.push(PrepareOKPayload::from_grpc(&_payload));
                        }
                        
                    },
                    Err(_) => todo!(),
                }
            });
        }
        reply_set
    }

    fn execute(&self) {
        info!("Executing");
        let epaxos_logic = self.epaxos_logic.lock().unwrap();
        //epaxos_logic.execute();
    }
}


// Handle message function
impl EpaxosService for EpaxosServerImpl {
    fn write(
        &mut self,
        ctx: ::grpcio::RpcContext,
        req: super::epaxos::WriteRequest,
        sink: ::grpcio::UnarySink<WriteResponse>,
    ) {
        let self_inner = Arc::<EpaxosServerInner>::clone(&self.inner);
        let task = async move {
            let mut r = grpc_service::WriteResponse::new();
            if self_inner.consensus(&WriteRequest::from_grpc(&req)) {
                (*self_inner.store.lock().unwrap())
                    .insert(req.get_key().to_owned(), req.get_value());
                info!("DONE my store: {:#?}", self_inner.store.lock().unwrap());
                info!("Consensus successful. Sending a commit to client\n\n\n\n.");
                r.set_commit(true);
            } else {
                info!("Consensus failed. Notifying client.");
                r.set_commit(false);
            }
            Ok(r)
        };

        super::util::spawn_grpc_task(sink, task);
    }

    fn read(
        &mut self,
        ctx: ::grpcio::RpcContext,
        req: super::epaxos::ReadRequest,
        sink: ::grpcio::UnarySink<ReadResponse>,
    ) {
        info!("read");
        let self_inner = Arc::<EpaxosServerInner>::clone(&self.inner);
        let task = async move {
            info!("Received a read request with key = {}", req.get_key());
            // TODO: When to run execute is a question.
            //self_inner.execute();
            let mut r = grpc_service::ReadResponse::new();
            let res_set = &*self_inner.store.lock().unwrap();
            let req_key = req.get_key();
            if res_set.get(req_key).is_none() {
                r.set_value(i32::MAX);
            } else {
                r.set_value(*res_set.get(req_key).unwrap());
            }
            Ok(r)
        };
        super::util::spawn_grpc_task(sink, task);
    }

    fn pre_accept(
        &mut self,
        ctx: ::grpcio::RpcContext,
        req: super::epaxos::Payload,
        sink: UnarySink<super::epaxos::Payload>,
    ) {
        info!("Received PreAccept");
        let self_inner = Arc::<EpaxosServerInner>::clone(&self.inner);
        let task = async move {
            let mut epaxos_logic = self_inner.epaxos_logic.lock().unwrap();
            let request = PreAccept(Payload::from_grpc(&req));
            let r = epaxos_logic.pre_accept_(request);
            Ok(r.0.to_grpc())
        };
        super::util::spawn_grpc_task(sink, task);
    }

    fn accept(
        &mut self,
        ctx: ::grpcio::RpcContext,
        req: super::epaxos::Payload,
        sink: ::grpcio::UnarySink<AcceptOKPayload>,
    ) {
        info!("Accept");
        let self_inner = Arc::<EpaxosServerInner>::clone(&self.inner);
        let task = async move {
            let mut epaxos_logic = self_inner.epaxos_logic.lock().unwrap();
            let request = Accept(Payload::from_grpc(&req));
            let r = epaxos_logic.accept_(request);
            Ok(r.0.to_grpc())
        };
        super::util::spawn_grpc_task(sink, task);
    }

    fn commit(
        &mut self,
        ctx: ::grpcio::RpcContext,
        req: super::epaxos::Payload,
        sink: ::grpcio::UnarySink<Empty>,
    ) {
        info!("Commit");
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

    fn prepare(
        &mut self,
        ctx: ::grpcio::RpcContext,
        req: super::epaxos::Payload,
        sink: ::grpcio::UnarySink<super::epaxos::PrepareOKPayload>,
    ) {
        let self_inner = Arc::<EpaxosServerInner>::clone(&self.inner);
        // 1. get most recent ballot number epoch.x.Y accepted for instance L.i
        // 2. according to ballot return prepareok or nack.
        let task = async move {
            let mut epaxos_logic = self_inner.epaxos_logic.lock().unwrap();
            let p = Payload::from_grpc(&req);
            match epaxos_logic.decide_prepare_ok(p) {
                Ok(_) => todo!(),
                Err(_) => todo!(),
            }
        };
        super::util::spawn_grpc_task(sink, task);
    }
}
