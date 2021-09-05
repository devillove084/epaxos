#![allow(unused_variables)]
#![allow(incomplete_features)]

use std::collections::HashMap;
use std::ops::{AddAssign, Index, MulAssign};
use std::sync::atomic::AtomicU32;
use std::sync::{Arc, Mutex};

use crate::commit::EpaxosLogic;
use crate::config::{CHECKPOINT_PERIOD, DO_CHECKPOINTING, MAXBATCH};

use super::config::{REPLICAS_NUM, REPLICA_TEST, SLOW_QUORUM};
use super::epaxos::{self as grpc_service, AcceptOKPayload, Empty};
use super::epaxos_grpc::{EpaxosService, EpaxosServiceClient};
use super::message::*;
use grpcio::{ChannelBuilder, Environment, UnarySink};
use log::{error, info};

lazy_static! {
    //static ref HOSTNAME: Mutex<String> = Mutex::new(String::new());
    pub static ref CP_COUNTER: Mutex<u32> = Mutex::new(0);
    pub static ref CP_MARKER: Vec<Command> = Vec::new();
}

fn get_num(n: u32) -> u32 {
    n
}

#[derive(Clone)]
pub struct EpaxosServerImpl {
    pub inner: Arc<EpaxosServerInner>,
}

pub struct EpaxosServerInner {
    pub store: Arc<Mutex<HashMap<String, i32>>>,
    pub epaxos_logic: Arc<Mutex<EpaxosLogic>>,
    pub replicas: HashMap<ReplicaId, EpaxosServiceClient>,
    pub quorum_members: Vec<ReplicaId>,
    pub max_recv_ballot: u32,
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
            max_recv_ballot: 0,
        }
    }

    pub fn handle_propose(&mut self, command: ProposePayload) {
        //TODO： handle client retry

        let mut epaxos_logic = self.epaxos_logic.lock().unwrap();

        let mut batch_size = epaxos_logic.propose_chan.len() + 1;
        if batch_size > MAXBATCH {
            batch_size = MAXBATCH;
        }

        let mut inst_no = epaxos_logic
            .current_instance
            .get(epaxos_logic.info.id.0 as usize)
            .unwrap();
        epaxos_logic.current_instance[epaxos_logic.info.id.0 as usize] += 1;

        info!("Starting instance {:?}", inst_no);
        info!("Batching is {:?}", batch_size);

        let mut cmds: Vec<Command> = Vec::with_capacity(batch_size as usize);
        let mut proposals: Vec<ProposePayload> = Vec::with_capacity(batch_size as usize);
        for i in 0..batch_size {
            let prop = epaxos_logic.propose_chan.pop();
            cmds.insert(i as usize, prop.command);
            proposals.insert(i as usize, prop);
        }

        self.start_phase_one(
            &Instance {
                replica: epaxos_logic.info.id.0,
                slot: *inst_no,
            },
            0,
            proposals,
            cmds,
            batch_size,
        );
    }

    pub fn start_phase_one(
        &mut self,
        instance: &Instance,
        ballot: u32,
        proposals: Vec<ProposePayload>,
        cmds: Vec<Command>,
        batch_size: u32,
    ) {
        let mut epaxos_logic = self.epaxos_logic.lock().unwrap();

        let seq = 0;
        let deps: Vec<Instance> = Vec::with_capacity(epaxos_logic.info.id.0 as usize);

        let (seq, deps, _) = epaxos_logic.update_attributes(cmds, seq, deps, instance);

        // TODO: Does is insert by current replica id and parm instance?
        epaxos_logic.instance_entry_space.insert(
            *instance,
            InstanceEntry {
                ballot,
                command: cmds,
                seq,
                deps,
                instance: *instance,
                state: PayloadState { state: 0 },
                from_leader: CommandLeaderBookKeeping::default(),
            },
        );

        epaxos_logic.update_conflicts(cmds, epaxos_logic.info.id, instance, seq);

        if seq > epaxos_logic.max_seq {
            epaxos_logic.max_seq = seq + 1;
        }

        epaxos_logic.record_payload_metadata(self.instance_space.get(instance).unwrap());
        epaxos_logic.record_commands(cmds);
        epaxos_logic.sync();

        self.send_pre_accepts(instance, ballot, cmds, seq, deps);

        CP_COUNTER.lock().unwrap().add_assign(batch_size);

        if epaxos_logic.info.id.0 == 0
            && DO_CHECKPOINTING
            && CP_COUNTER.lock().unwrap().ge(&CHECKPOINT_PERIOD)
        {
            CP_COUNTER.lock().unwrap().mul_assign(0);

            epaxos_logic.current_instance[epaxos_logic.info.id.0 as usize] += 1;
            instance.slot += 1;
            epaxos_logic.max_seq += 1;

            for q in 0..epaxos_logic.info.n {
                //deps.push(epaxos_logic.current_instance[q] );
                let slot = epaxos_logic.current_instance[q] - 1;
                deps.push(Instance {
                    replica: epaxos_logic.info.id.0,
                    slot,
                });
            }

            // TODO: Change the instance to other things?
            epaxos_logic.instance_entry_space.insert(
                *instance,
                InstanceEntry {
                    ballot: 0,
                    command: CP_MARKER.to_vec(),
                    seq: epaxos_logic.max_seq,
                    deps,
                    instance: *instance,
                    state: PayloadState { state: 0 },
                    from_leader: CommandLeaderBookKeeping {
                        max_recv_ballot: 0,
                        prepare_oks: 0,
                        all_equal: true,
                        pre_accept_oks: 0,
                        accept_oks: 0,
                        nacks: 0,
                        original_deps: deps,
                        commited_deps: Vec::new(),
                        recovery_insts: RecoveryPayloadEntry::default(),
                        preparing: false,
                        trying_to_pre_accept: false,
                        possible_quorum: Vec::new(),
                        tpa_oks: 0,
                        commit_time: 0,
                        client_proposals: Vec::new(),
                    },
                },
            );

            epaxos_logic.lastest_cp_instance = instance.slot;
            epaxos_logic.lastest_cp_replica = instance.replica;

            epaxos_logic.clear_hashtables();
            epaxos_logic
                .record_payload_metadata(epaxos_logic.instance_entry_space.get(instance).unwrap());
            epaxos_logic.record_commands(cmds);
            epaxos_logic.sync();

            self.send_pre_accepts(instance, ballot, cmds, seq, deps);
        }
    }

    // handle preparereply
    // we only need to do consensus for write req
    fn consensus(&self, write_req: &WriteRequest) -> bool {
        //fn consensus(&self, command: &Command) -> bool {
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

    // fn explicit_prepare(&self, payload: &Payload) {
    //     let mut epaxos_logic = self.epaxos_logic.lock().unwrap();
    //     // Paper: where epoch.b.R is the highest ballot number Q is aware of in instance L.i
    //     let mut pre_payload = epaxos_logic.aware_ballot(payload);
    //     let mut replies = self.send_prepare(&pre_payload);

    //     match epaxos_logic.decide_prepare(replies, payload) {
    //         PrepareStage::Commit(_payload) => {
    //             self.send_commits(&_payload);
    //             epaxos_logic.committed(_payload);
    //         }
    //         PrepareStage::PaxosAccept(_payload) => {
    //             if self.send_accepts(&_payload) >= SLOW_QUORUM {
    //                 self.send_commits(&_payload);
    //                 epaxos_logic.committed(_payload);
    //             }
    //         }
    //         PrepareStage::PhaseOne(_payload) => {
    //             //TODO: start Phase 1 (at line 2) for γ at L.i,
    //             // start Phase 1 (at line 2) for no-op at L.i
    //             self.consensus(&payload.write_req);
    //         }
    //     }
    // }

    // send preaccept + handle preacceptreply
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
                // handle preaccept reply && preacceptok 这里不仅调用handlereply 还有 handleok
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

    fn start_recovery_instance(&self, payload: &mut Payload) {
        let mut epaxos_logic = self.epaxos_logic.lock().unwrap();
        epaxos_logic._recovery_instance(payload);
        self.send_prepare(payload);
    }

    // fn send_prepare(&self, payload: &Payload) -> Vec<PrepareOKPayload> {
    //     info!("Send Explicit Prepare");
    //     let mut reply_set = Vec::new();
    //     let mut epaxos_logic = self.epaxos_logic.lock().unwrap();
    //     if self.quorum_members.len() < 1 {
    //         log::error!("Not enough replicas alive");
    //     }

    //     for replica_id in self.quorum_members.iter() {
    //         // Detect every peers in my brain is alive or not.
    //         if !epaxos_logic.info.alive.get(&replica_id.0).unwrap() {
    //             continue;
    //         }
    //         smol::block_on(async {
    //             let prepare_ok = self
    //                 .replicas
    //                 .get(replica_id)
    //                 .unwrap()
    //                 .prepare_async(&payload.to_grpc());
    //             let value = prepare_ok.unwrap().await;
    //             match value {
    //                 Ok(_payload) => {
    //                     if _payload.ballot == self.max_recv_ballot {
    //                         reply_set.push(PrepareOKPayload::from_grpc(&_payload));
    //                     }
    //                 }
    //                 Err(e) => panic!("Send prepare failed about: {}", e),
    //             }
    //         });
    //     }
    //     reply_set
    // }

    fn execute(&self) {
        info!("Executing");
        let epaxos_logic = self.epaxos_logic.lock().unwrap();
        //epaxos_logic.execute();
    }
}

// Handle message function
impl EpaxosService for EpaxosServerImpl {
    // fn write(
    //     &mut self,
    //     ctx: ::grpcio::RpcContext,
    //     req: super::epaxos::Propose,
    //     sink: UnarySink<super::epaxos::ProposeReply>,
    // ) {
    //     let self_inner = Arc::<EpaxosServerInner>::clone(&self.inner);
    //     let task = async move {
    //         let mut r = grpc_service::WriteResponse::new();
    //         if self_inner.consensus(&WriteRequest::from_grpc(&req)) {
    //             (*self_inner.store.lock().unwrap())
    //                 .insert(req.get_key().to_owned(), req.get_value());
    //             info!("DONE my store: {:#?}", self_inner.store.lock().unwrap());
    //             info!("Consensus successful. Sending a commit to client\n\n\n\n.");
    //             r.set_commit(true);
    //         } else {
    //             info!("Consensus failed. Notifying client.");
    //             r.set_commit(false);
    //         }
    //         Ok(r)
    //     };

    //     super::util::spawn_grpc_task(sink, task);
    // }

    // fn read(
    //     &mut self,
    //     ctx: ::grpcio::RpcContext,
    //     req: super::epaxos::Propose,
    //     sink: UnarySink<super::epaxos::ProposeReply>,
    // ) {
    //     info!("read");
    //     let self_inner = Arc::<EpaxosServerInner>::clone(&self.inner);
    //     let task = async move {
    //         info!("Received a read request with key = {}", req.get_key());
    //         // TODO: When to run execute is a question.
    //         //self_inner.execute();
    //         let mut r = grpc_service::ReadResponse::new();
    //         let res_set = &*self_inner.store.lock().unwrap();
    //         let req_key = req.get_key();
    //         if res_set.get(req_key).is_none() {
    //             r.set_value(i32::MAX);
    //         } else {
    //             r.set_value(*res_set.get(req_key).unwrap());
    //         }
    //         Ok(r)
    //     };
    //     super::util::spawn_grpc_task(sink, task);
    // }

    // handle preaccept
    fn pre_accept(
        &mut self,
        ctx: ::grpcio::RpcContext,
        req: super::epaxos::PreAcceptPayload,
        sink: UnarySink<super::epaxos::PreAcceptReplyPayload>,
    ) {
        info!("Received PreAccept");
        let self_inner = Arc::<EpaxosServerInner>::clone(&self.inner);
        let task = async move {
            let mut epaxos_logic = self_inner.epaxos_logic.lock().unwrap();
            let pre_accept_request = PreAccept(PreAcceptPayload::from_grpc(&req));
            let mut inst = epaxos_logic.instance_entry_space.get(&Instance {
                replica: pre_accept_request.0.leader_id,
                slot: pre_accept_request.0.instance.slot,
            });
            let (mut seq, mut deps, mut changed) = epaxos_logic.update_attributes(
                pre_accept_request.0.command,
                pre_accept_request.0.seq,
                pre_accept_request.0.deps,
                pre_accept_request.0.instance,
            );
            let mut state_change = PayloadState{ state_num: 1 };
            if changed {
                state_change = PayloadState{ state_num: 0 };
            }

            let mut uncommitted = false;
            for q in 0..epaxos_logic.info.n {
                if deps[q] > epaxos_logic.commited_upto_instance[q] {
                    uncommitted = true;
                    break;
                }
            }

            match inst {
                Some(instance) => {
                    if pre_accept_request.0.seq > epaxos_logic.max_seq {
                        epaxos_logic.max_seq = pre_accept_request.0.seq + 1;
                    }

                    // 3 -> committed && 4 -> executed
                    if instance.state.state_num == 3 || instance.state.state_num == 4 {
                        // reordered handling of commit/accept and pre-accept
                        if instance.command.len() == 0 {
                            epaxos_logic.instance_entry_space[&Instance {
                                replica: pre_accept_request.0.leader_id,
                                slot: pre_accept_request.0.instance.slot,
                            }]
                                .command = pre_accept_request.0.command;
                            epaxos_logic.update_conflicts(
                                pre_accept_request.0.command,
                                pre_accept_request.0.instance,
                                pre_accept_request.0.seq,
                            );
                            epaxos_logic.sync();
                            // How to return here??;
                        }
                    }

                    if pre_accept_request.0.instance.slot
                        >= epaxos_logic.current_instance
                            [pre_accept_request.0.instance.replica as usize]
                    {
                        epaxos_logic.current_instance
                            [pre_accept_request.0.instance.replica as usize] =
                            pre_accept_request.0.instance.slot + 1;
                    }



                    if pre_accept_request.0.ballot < instance.ballot {
                        self.send_pre_accept_reply(pre_accept_request.0.leader_id, PreAcceptReplyPayload{
                            instance: pre_accept_request.0.instance,
                            ok: 0,
                            ballot: instance.ballot,
                            command: instance.command,
                            seq: instance.seq,
                            deps: instance.deps,
                            commited_deps: epaxos_logic.commited_upto_instance,
                        });
                    }else {
                        instance.command = pre_accept_request.0.command;
                        instance.seq = seq;
                        instance.deps = deps;
                        instance.ballot = pre_accept_request.0.ballot;
                        instance.state = state_change;
                    }
                }
                None => {
                    epaxos_logic.instance_entry_space.insert( pre_accept_request.0.instance, InstanceEntry {
                        ballot: pre_accept_request.0.ballot,
                        command: pre_accept_request.0.command,
                        seq,
                        deps,
                        instance: pre_accept_request.0.instance,
                        state: state_change,
                        from_leader: CommandLeaderBookKeeping::default(),
                    });
                },
            }
            
            epaxos_logic.update_conflicts(pre_accept_request.0.command, pre_accept_request.0.instance, pre_accept_request.0.seq);
            epaxos_logic.record_payload_metadata(epaxos_logic.instance_entry_space.get(&pre_accept_request.0.instance).unwrap());
            epaxos_logic.record_commands(pre_accept_request.0.command);
            epaxos_logic.sync();
            
            // checkpoint && update latest check point info 
            if pre_accept_request.0.command.len() == 0 {
                epaxos_logic.lastest_cp_replica = pre_accept_request.0.instance.replica;
                epaxos_logic.lastest_cp_instance = pre_accept_request.0.instance.slot;

                epaxos_logic.clear_dep_hash_tables();
            }

            if changed || uncommitted || pre_accept_request.0.instance.replica != pre_accept_request.0.leader_id || !is_initial_ballot(pre_accept_request.0.ballot) {
                let pre_accept_reply_payload = PreAcceptReplyPayload {
                    instance: pre_accept_request.0.instance,
                    ok: 1,
                    ballot: pre_accept_request.0.ballot,
                    command: pre_accept_request.0.command,
                    seq,
                    deps,
                    commited_deps: epaxos_logic.commited_upto_instance,
                };
                return Ok(pre_accept_reply_payload.to_grpc());
            } else {
                let pok = PreAcceptOKPayload {
                    instance: pre_accept_request.0.instance,
                };
                self_inner.send_pre_accept_ok(pok);
            }

            // let r = epaxos_logic.pre_accept_(pre_accept_request);
            // Ok(r.0.to_grpc())
        };
        super::util::spawn_grpc_task(sink, task);
    }

    // handle accept
    fn accept(
        &mut self,
        ctx: ::grpcio::RpcContext,
        req: super::epaxos::AcceptPayload,
        sink: UnarySink<super::epaxos::AcceptReplyPayload>, // 原本是AcceptOKPayload
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

    // handle commit
    fn commit(
        &mut self,
        ctx: ::grpcio::RpcContext,
        req: super::epaxos::CommitPayload,
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
        req: super::epaxos::PreparePayload,
        sink: ::grpcio::UnarySink<super::epaxos::PrepareReplyPayload>,
    ) {
        let self_inner = Arc::<EpaxosServerInner>::clone(&self.inner);
        // 1. get most recent ballot number epoch.x.Y accepted for instance L.i
        // 2. according to ballot return prepareok or nack.
        let task = async move {
            let mut epaxos_logic = self_inner.epaxos_logic.lock().unwrap();
            let p = Payload::from_grpc(&req);
            match epaxos_logic.reply_prepare_ok(p) {
                Ok(_) => todo!(),
                Err(_) => todo!(),
            }
        };
        super::util::spawn_grpc_task(sink, task);
    }

    fn commitshort(
        &mut self,
        ctx: grpcio::RpcContext,
        req: crate::epaxos::CommitShortPayload,
        sink: grpcio::UnarySink<crate::epaxos::Empty>,
    ) {
        todo!()
    }

    // handle propose
    fn propose(
        &mut self,
        ctx: grpcio::RpcContext,
        req: crate::epaxos::ProposePayload,
        sink: grpcio::UnarySink<crate::epaxos::ProposeReplyPayload>,
    ) {
        todo!()
    }

    fn try_pre_accept(
        &mut self,
        ctx: grpcio::RpcContext,
        req: crate::epaxos::TryPreAcceptPayload,
        sink: grpcio::UnarySink<crate::epaxos::TryPreAcceptReplyPayload>,
    ) {
        todo!()
    }
}
