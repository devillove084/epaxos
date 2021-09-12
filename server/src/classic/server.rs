#![allow(unused_variables)]
#![allow(incomplete_features)]

use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::ops::{AddAssign, Index, MulAssign};
use std::sync::atomic::AtomicU32;

use futures::TryFutureExt;
use smol::channel::{Sender,Receiver};

use std::sync::{Arc, Mutex};
use std::thread;

use crate::config::{CHECKPOINT_PERIOD, DO_CHECKPOINTING, MAXBATCH};
use crate::epaxos_info::{equal, EpaxosLogic};

use super::config::{REPLICAS_NUM, REPLICA_TEST, SLOW_QUORUM};
use super::epaxos::{self as grpc_service, Empty};
use super::epaxos_grpc::{EpaxosService, EpaxosServiceClient};
use super::message::*;
use grpcio::{ChannelBuilder, ClientUnaryReceiver, Environment, UnarySink};
use log::{error, info};

lazy_static! {
    //static ref HOSTNAME: Mutex<String> = Mutex::new(String::new());
    pub static ref CP_COUNTER: Mutex<u32> = Mutex::new(0);
    pub static ref CP_MARKER: Vec<Command> = Vec::new();
    // Default queue buffer is vec
    // TODO: maybe better?
    pub static ref QUEUE_BUFFER: Mutex<VecDeque<ProposePayload>> = Mutex::new(VecDeque::new());

    //pub static ref PREACCEPT_RESULT_CHANNEL: (Receiver<ClientUnaryReceiver<PreAcceptReplyPayload>>, Sender<ClientUnaryReceiver<PreAcceptReplyPayload>>) = smol::channel::unbounded();
    // pub static ref PREPARE_RESULT_CHANNEL: (Receiver<ClientUnaryReceiver<PrepareReplyPayload>>, Sender<ClientUnaryReceiver<PrepareReplyPayload>>) = channel();
    // pub static ref TRY_PRE_ACCEPT_RESULT_CHANNEL: (Receiver<ClientUnaryReceiver<()>>, Sender<ClientUnaryReceiver<()>>) = channel();
    // pub static ref ACCEPT_RESULT_CHANNEL: (Receiver<ClientUnaryReceiver<()>>, Sender<ClientUnaryReceiver<()>>) = channel();
    // pub static ref COMMIT_RESULT_CHANNEL: (Receiver<ClientUnaryReceiver<()>>, Sender<ClientUnaryReceiver<()>>) = channel();
    // pub static ref COMMIT_SHORT_RESULT_CHANNEL: (Receiver<ClientUnaryReceiver<()>>, Sender<ClientUnaryReceiver<()>>) = channel();
}

#[derive(Clone)]
pub struct EpaxosServerImpl {
    pub inner: Arc<EpaxosServerInner>,
}

pub struct EpaxosServerInner {
    pub store: Arc<Mutex<HashMap<String, i32>>>,
    pub epaxos_logic: Arc<Mutex<EpaxosLogic>>,
    pub replicas: HashMap<usize, EpaxosServiceClient>,
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

    pub fn start_phase_one(
        &mut self,
        instance: &Instance,
        ballot: u32,
        proposal: ProposePayload,
        cmds: Vec<Command>,
        batch_size: u32,
    ) {
        let mut epaxos_logic = self.epaxos_logic.lock().unwrap();
        //let cmds = proposal.command;
        let seq = 0;
        let deps: Vec<u32> = Vec::with_capacity(epaxos_logic.info.id.0 as usize);

        let (mut seq, mut deps, _) = epaxos_logic.update_attributes(cmds, seq, deps, instance);


        // TODO: Does is insert by current replica id and parm instance?
        epaxos_logic.instance_entry_space.insert(
            *instance,
            InstanceEntry {
                ballot,
                command: Some(proposal.command),
                seq,
                deps,
                instance: *instance,
                state: State::PreAccepted,
                from_leader: Some(CommandLeaderBookKeeping {
                    client_proposals: Some(proposal),
                    max_recv_ballot: 0,
                    prepare_oks: 0,
                    all_equal: true,
                    pre_accept_oks: 0,
                    accept_oks: 0,
                    nacks: 0,
                    original_deps: deps,
                    commited_deps: vec![u32::MAX; deps.len()],
                    recovery_insts: None,
                    preparing: false,
                    trying_to_pre_accept: false,
                    possible_quorum: Vec::new(),
                    tpa_oks: 0,
                    commit_time: 0,
                }),
            },
        );

        epaxos_logic.update_conflicts(cmds, instance, seq);
        //epaxos_logic.update_conflicts(cmds, &instance, seq);

        if seq > epaxos_logic.max_seq {
            epaxos_logic.max_seq = seq + 1;
        }

        epaxos_logic.record_payload_metadata(epaxos_logic.instance_entry_space.get(instance).unwrap());
        epaxos_logic.record_commands(cmds);
        epaxos_logic.sync();

        //self.send_pre_accepts(instance, ballot, cmds, seq, deps);
        self.broadcast_pre_accepts(instance, ballot, cmds, seq, deps);

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
                deps.push(slot);
            }

            epaxos_logic.instance_entry_space.insert(
                *instance,
                InstanceEntry {
                    ballot: 0,
                    command: Some(CP_MARKER.to_vec()),
                    seq: epaxos_logic.max_seq,
                    deps,
                    instance: *instance,
                    state: Some(State::PreAccepted),
                    from_leader: Some(RefCell::new(CommandLeaderBookKeeping {
                        max_recv_ballot: 0,
                        prepare_oks: 0,
                        all_equal: true,
                        pre_accept_oks: 0,
                        accept_oks: 0,
                        nacks: 0,
                        original_deps: deps,
                        commited_deps: Vec::new(),
                        recovery_insts: None,
                        preparing: false,
                        trying_to_pre_accept: false,
                        possible_quorum: Vec::new(),
                        tpa_oks: 0,
                        commit_time: 0,
                        client_proposals: None,
                    })),
                },
            );

            epaxos_logic.lastest_cp_instance = instance.slot;
            epaxos_logic.lastest_cp_replica = instance.replica;

            epaxos_logic.clear_hashtables();
            epaxos_logic
                .record_payload_metadata(epaxos_logic.instance_entry_space.get(instance).unwrap());
            epaxos_logic.record_commands(cmds);
            epaxos_logic.sync();

            self.broadcast_pre_accepts(instance, ballot, cmds, seq, deps);
            //self.send_pre_accepts(instance, ballot, cmds, seq, deps);
        }
    }

    // handle preparereply
    // we only need to do consensus for write req
    // fn consensus(&self, write_req: &WriteRequest) -> bool {
    //     //fn consensus(&self, command: &Command) -> bool {
    //     info!("Starting consensus");
    //     let mut epaxos_logic = self.epaxos_logic.lock().unwrap();
    //     let payload = epaxos_logic.lead_consensus(write_req.clone());
    //     let pre_accept_oks = self.send_pre_accepts(&payload);

    //     match epaxos_logic.decide_path(pre_accept_oks, &payload) {
    //         Path::Fast(payload_) => {
    //             // Send Commit message to F
    //             self.send_commits(&payload_);
    //             epaxos_logic.committed(payload_);
    //             return true;
    //         }
    //         Path::Slow(payload_) => {
    //             // Start Paxos-Accept stage
    //             // Send Accept message to F
    //             epaxos_logic.accepted(payload_.clone());
    //             if self.send_accepts(&payload_) >= SLOW_QUORUM {
    //                 self.send_commits(&payload_);
    //                 epaxos_logic.committed(payload_);
    //                 return true;
    //             }
    //             return false;
    //         }
    //     }
    // }

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

    pub fn broadcast_pre_accepts(
        &self,
        instance: &Instance,
        ballot: u32,
        cmds: Vec<Command>,
        seq: u32,
        deps: Vec<u32>,
    ) {
        info!("send PreAccepts");
        let mut epaxos_logic = self.epaxos_logic.lock().unwrap();
        let mut pa = PreAcceptPayload {
            leader_id: epaxos_logic.info.id.0,
            instance: *instance,
            ballot,
            command: cmds,
            seq,
            deps,
        };

        let mut n = epaxos_logic.info.n - 1;
        if epaxos_logic.info.thrifty {
            n = epaxos_logic.info.n / 2;
        }

        // TODO: use a queue
        let mut result = Vec::new();
        for replica_id in 0..n {
            if !epaxos_logic.info.alive
                [&epaxos_logic.info.preferred_peer_order[replica_id]]
            {
                continue;
            }

            let task = smol::spawn(async {
                let payload = self
                    .replicas
                    .get(&replica_id)
                    .unwrap()
                    .pre_accept_async_opt(&pa.to_grpc(), grpcio::CallOption::default())
                    .unwrap_or_else(|e| {
                        panic!("Panic on unwrap pre_accept,{}", e);
                    });
                
                let res = payload.await;
                match res {
                    Ok(pre_accept_reply_payload) => {
                        self.handle_pre_accept_reply(&PreAcceptReplyPayload::from_grpc(&pre_accept_reply_payload));
                    },
                    Err(e) => {
                        error!("Error on received pre_accept_reply {:?}", &e);
                    },
                }
            });

            
            result.push(task);
            info!("send pre accept payload");
        }

        // XXX: Produce better?
        for t in result {
            smol::block_on(async {
                t.await;
            });
        }
        
    }

    pub fn broadcast_prepare(&self, instance: &Instance, ballot: u32) {
        let mut epaxos_logic = self.epaxos_logic.lock().unwrap();
        let mut prepare_paylod = PreparePayload {
            leader_id: epaxos_logic.info.id.0,
            ballot,
            instance: *instance,
        };

        let mut n = epaxos_logic.info.n - 1;
        if epaxos_logic.info.thrifty {
            n = epaxos_logic.info.n / 2;
        }

        let mut q: usize = epaxos_logic.info.id.0 as usize;
        let sent: usize = 0;
        let mut result = Vec::new();
        while sent < n {
            q = (q + 1) % epaxos_logic.info.n;
            if q == epaxos_logic.info.id.0 as usize {
                info!("Not enough replicas alive!");
            }

            if !epaxos_logic.info.alive[&(q as u32)] {
                continue;
            }

            let task = smol::spawn(async {
                let payload = self
                .replicas
                .get(&sent)
                .unwrap()
                .prepare_async_opt(&prepare_paylod.to_grpc(), grpcio::CallOption::default())
                .unwrap_or_else(|e| {
                    panic!("Panic on unwrap accept,{}", e);
                });

                let result = payload.await;
                match result {
                    Ok(prepare_reply_payload) => {
                        self.handle_prepare_reply(&PrepareReplyPayload::from_grpc(&prepare_reply_payload));
                    },
                    Err(error) => {
                        error!("Error when get the prepare reply payload{:?}", &error);
                    },
                }
            });

            result.push(task);
            info!("Send Prepare payload and received prepare reply payloads");
            sent += 1;
        }
        
        // XXX: Produce better?
        for t in result {
            smol::block_on(async {
                t.await
            });
        }
    }

    pub fn broadcast_try_pre_accept(
        &self,
        instance: &Instance,
        ballot: u32,
        cmds: Vec<Command>,
        seq: u32,
        deps: Vec<u32>,
    ) {
        let mut epaxos_logic = self.epaxos_logic.lock().unwrap();
        let mut try_pre_accept_payload = TryPreAcceptPayload {
            leader_id: epaxos_logic.info.id.0,
            instance: *instance,
            ballot,
            command: cmds,
            seq,
            deps,
        };

        let mut result = Vec::new();
        for q in 0..epaxos_logic.info.n {
            if q == epaxos_logic.info.id.0 as usize {
                continue;
            }
            if !epaxos_logic.info.alive[&(q as u32)] {
                continue;
            }

            let task = smol::spawn(async {
                let payload = self
                    .replicas
                    .get(&q)
                    .unwrap()
                    .try_pre_accept_async_opt(
                        &try_pre_accept_payload.to_grpc(),
                        grpcio::CallOption::default(),
                    )
                    .unwrap_or_else(|e| {
                        panic!("Panic on unwrap try_pre_accept,{}", e);
                    });
                let res = payload.await;
                match res {
                    Ok(try_pre_accept_payload) => {
                        self.handle_try_pre_accept_reply(&TryPreAcceptReplyPayload::from_grpc(&try_pre_accept_payload));
                    },
                    Err(e) => {
                        error!("Error on receive try_pre_accept payload{:?}", &e);
                    },
                }
            });
            result.push(task);
            info!("send try pre accept payload");
        }

        for t in result {
            smol::block_on(async {
                t.await;
            });
        }
    }

    pub fn broadcast_accept(
        self,
        instance: &Instance,
        ballot: u32,
        count: u32,
        seq: u32,
        deps: Vec<u32>,
    ) {
        let mut epaxos_logic = self.epaxos_logic.lock().unwrap();
        let accept_payload = AcceptPayload {
            leader_id: epaxos_logic.info.id.0,
            instance: *instance,
            ballot,
            count,
            seq,
            deps,
        };

        let mut n = epaxos_logic.info.n - 1;
        if epaxos_logic.info.thrifty {
            n = epaxos_logic.info.n / 2;
        }

        let sent: usize = 0;
        //let mut result = Vec::new();
        for q in 0..n {
            if !epaxos_logic.info.alive[&epaxos_logic.info.preferred_peer_order[q]] {
                continue;
            }

            smol::block_on(async {
                let payload = self
                    .replicas
                    .get(&(epaxos_logic.info.preferred_peer_order[q] as usize))
                    .unwrap()
                    .accept_async_opt(&accept_payload.to_grpc(), grpcio::CallOption::default())
                    .unwrap_or_else(|e| {
                        panic!("Panic on unwrap accept,{}", e);
                    });
                let res = payload.await;
                match res {
                    Ok(accept_reply_payload) => {
                        self.handle_accept_reply(&AcceptReplyPayload::from_grpc(&accept_reply_payload));
                    },
                    Err(e) => {
                        error!("Error on receive accept reply payload");
                    },
                }
            });
            //result.push(task);
            info!("send accept payload");
        }

        // for t in result {
        //     smol::block_on(async {
        //         t.await;
        //     });
        // }
    }

    pub fn broadcast_commit(
        self,
        instance: &Instance,
        cmds: Vec<Command>,
        seq: u32,
        deps: Vec<u32>,
    ) {
        let mut epaxos_logic = self.epaxos_logic.lock().unwrap();
        let commit_payload = CommitPayload {
            leader_id: epaxos_logic.info.id.0,
            instance: *instance,
            command: cmds,
            seq,
            deps,
        };

        let commit_short_payload = CommitShortPayload {
            leader_id: epaxos_logic.info.id.0,
            instance: *instance,
            count: cmds.len() as u32,
            seq,
            deps,
        };

        //let mut result = Vec::new();
        let mut sent: usize = 0;
        for q in 0..epaxos_logic.info.n - 1 {
            if !epaxos_logic.info.alive[&epaxos_logic.info.preferred_peer_order[q]] {
                continue;
            }

            if epaxos_logic.info.thrifty && sent >= epaxos_logic.info.n / 2 {
                smol::block_on(async {
                    let payload = self
                    .replicas
                    .get(&(epaxos_logic.info.preferred_peer_order[q] as usize))
                    .unwrap()
                    .commit_async_opt(&commit_payload.to_grpc(), grpcio::CallOption::default())
                    .unwrap_or_else(|e| {
                        panic!("Panic on unwrap commit,{}", e);
                    });

                    let res = payload.await;
                    match res {
                        Ok(empty_info) => {
                            info!("Just commit");
                        },
                        Err(e) => {
                            error!("Error on commit send, empty message miss");
                        },
                    }

                });

                // let sender = COMMIT_RESULT_CHANNEL.1.clone();
                // sender.send(payload);
                info!("send commit payload");
            } else {
                smol::block_on(async {
                    let payload = self
                    .replicas
                    .get(&(epaxos_logic.info.preferred_peer_order[q] as usize))
                    .unwrap()
                    .commitshort_async_opt(
                        &commit_short_payload.to_grpc(),
                        grpcio::CallOption::default(),
                    )
                    .unwrap_or_else(|e| {
                        panic!("Panic on unwrap commit_short,{}", e);
                    });

                    let res = payload.await;
                    match res {
                        Ok(empty_info) => {
                            info!("Just commit_short");
                        },
                        Err(e) => {
                            error!("Error on commit_short send, empty message miss");
                        },
                    }
                });

                
                // result.push(task);
                // let sender = COMMIT_SHORT_RESULT_CHANNEL.1.clone();
                // sender.send(payload);
                info!("send commit_short payload");
            }
        }

        // for t in result {
        //     smol::block_on(async {
        //         t.await;
        //     });
        // }
    }

    pub fn handle_prepare_reply(&self, preply: &PrepareReplyPayload) {
        let mut epaxos_logic = self.epaxos_logic.lock().unwrap();
        let mut inst = epaxos_logic
            .instance_entry_space
            .get(&preply.instance)
            .unwrap();

        if inst.from_leader.is_none() || !inst.from_leader.unwrap().borrow().preparing {
            // TODO: fix return
            // we've moved on -- these are delayed replies, so just ignore
            // TODO: should replies for non-current ballots be ignored?
            return;
        }

        if preply.ok == 0 {
            // TODO: there is probably another active leader, back off and retry later
            inst.from_leader.as_ref().unwrap().borrow_mut().nacks += 1;
        }

        //Got an ACK (preply.OK == TRUE)

        inst.from_leader.as_ref().unwrap().borrow_mut().prepare_oks += 1;

        if inst.state.unwrap() == State::Committed || inst.state.unwrap() == State::Executed {
            epaxos_logic.instance_entry_space.insert(
                preply.instance,
                InstanceEntry {
                    ballot: inst.ballot,
                    command: Some(preply.command),
                    seq: preply.seq,
                    deps: preply.deps,
                    instance: preply.instance,
                    state: Some(State::Committed),
                    from_leader: None,
                },
            );

            //broadcastcommit
            //self.send_commits(preply.Instance, inst.Cmds, preply.Seq, preply.Deps);
            self.broadcast_commit(&preply.instance, inst.command.unwrap(), preply.seq, preply.deps);
            // TODO: does it really need to return here? fix return
            return;
        }

        if preply.state == State::Accepted {
            if inst.from_leader.unwrap().borrow().recovery_insts.is_none()
                || inst.from_leader.unwrap().borrow().max_recv_ballot < preply.ballot
            {
                inst.from_leader.unwrap().borrow_mut().recovery_insts = Some(RecoveryPayloadEntry {
                    command: preply.command,
                    state: preply.state,
                    seq: preply.seq,
                    deps: preply.deps,
                    pre_accept_count: 0,
                    command_leader_response: false,
                });
                inst.from_leader.unwrap().borrow_mut().max_recv_ballot = preply.ballot;
            }
        }

        if (preply.state == State::PreAccepted && preply.state == State::PreAcceptedEq)
            && (inst.from_leader.unwrap().borrow().recovery_insts.is_none()
                || inst.from_leader.unwrap().borrow().recovery_insts.unwrap().state < State::Accepted)
        {
            if inst.from_leader.unwrap().borrow().recovery_insts.is_none() {
                inst.from_leader.unwrap().borrow_mut().recovery_insts = Some(RecoveryPayloadEntry {
                    command: preply.command,
                    state: preply.state,
                    seq: preply.seq,
                    deps: preply.deps,
                    pre_accept_count: 1,
                    command_leader_response: false,
                });
            } else if preply.seq == inst.seq && equal(preply.deps, inst.deps) {
                inst.from_leader
                    .unwrap()
                    .borrow_mut()
                    .recovery_insts
                    .unwrap()
                    .pre_accept_count += 1;
            } else if preply.state == State::PreAccepted {
                // If we get different ordering attributes from pre-acceptors, we must go with the ones
                // that agreed with the initial command leader (in case we do not use Thrifty).
                // This is safe if we use thrifty, although we can also safely start phase 1 in that case.
                inst.from_leader.as_ref().unwrap().borrow_mut().recovery_insts = Some(RecoveryPayloadEntry {
                    command: preply.command.clone(),
                    state: preply.state,
                    seq: preply.seq,
                    deps: preply.deps.clone(),
                    pre_accept_count: 1,
                    command_leader_response: false,
                });
            }

            if preply.accept_id == preply.instance.replica {
                inst.from_leader
                    .unwrap()
                    .borrow_mut()
                    .recovery_insts
                    .unwrap()
                    .command_leader_response = true;
            }
        }

        if inst.from_leader.unwrap().borrow().prepare_oks < epaxos_logic.info.n as u32 / 2 {
            // return or maybe record this as log
            // TODO: fix return
            return;
        }

        let mut recover_payload = inst.from_leader.unwrap().borrow().recovery_insts;
        match recover_payload {
            Some(ir) => {
                //at least one replica has (pre-)accepted this instance
                if ir.state == State::Accepted
                    || (!ir.command_leader_response
                        && ir.pre_accept_count >= epaxos_logic.info.n as u32 / 2
                        && (epaxos_logic.info.thrifty || ir.state == State::PreAcceptedEq))
                {
                    //safe to go to Accept phase
                    inst.command = Some(ir.command);
                    inst.seq = ir.seq;
                    inst.deps = ir.deps;
                    inst.state = Some(State::Accepted);
                    inst.from_leader.unwrap().borrow_mut().preparing = false;
                    // broadcast accepts
                    self.broadcast_accept(&preply.instance, inst.ballot, inst.command.unwrap().len() as u32, inst.seq, inst.deps);
                    // self.send_accepts(
                    //     preply.instance,
                    //     inst.ballot,
                    //     inst.command.len(),
                    //     inst.seq,
                    //     inst.deps,
                    // );
                } else if !ir.command_leader_response
                    && ir.pre_accept_count >= (epaxos_logic.info.n as u32 / 2 + 1) / 2
                {
                    //send TryPreAccepts
                    //but first try to pre-accept on the local replica
                    inst.from_leader.unwrap().borrow_mut().pre_accept_oks = 0;
                    inst.from_leader.unwrap().borrow_mut().nacks = 0;
                    inst.from_leader.unwrap().borrow_mut().possible_quorum =
                        Vec::with_capacity(epaxos_logic.info.n);

                    for q in 0..epaxos_logic.info.n {
                        inst.from_leader.unwrap().borrow_mut().possible_quorum[q] = true;
                    }

                    let (mut conflict, mut replica_q, mut instance_i) = epaxos_logic
                        .find_pre_accept_conflicts(ir.command, preply.instance, ir.seq, ir.deps);
                    if conflict {
                        if epaxos_logic
                            .instance_entry_space
                            .get(&Instance {
                                replica: replica_q,
                                slot: instance_i,
                            })
                            .unwrap()
                            .state.unwrap()
                            == State::Committed
                        {
                            // start Phase1 in the initial leader's instance
                            self.start_phase_one(
                                &preply.instance,
                                inst.ballot,
                                inst.from_leader.unwrap().borrow().client_proposals.unwrap(),
                                ir.command,
                                ir.command.len() as u32,
                            );
                            return;
                        } else {
                            inst.from_leader.unwrap().borrow_mut().nacks = 1;
                            inst.from_leader.unwrap().borrow_mut().possible_quorum
                                [epaxos_logic.info.id.0 as usize] = false;
                        }
                    } else {
                        inst.command = Some(ir.command);
                        inst.seq = ir.seq;
                        inst.deps = ir.deps;
                        inst.state = Some(State::PreAccepted);
                        inst.from_leader.unwrap().borrow_mut().pre_accept_oks = 1;
                    }

                    inst.from_leader.unwrap().borrow_mut().preparing = false;
                    inst.from_leader.unwrap().borrow_mut().trying_to_pre_accept = true;
                    // self.send_try_pre_accept(
                    //     preply.instance,
                    //     inst.ballot,
                    //     inst.command,
                    //     inst.seq,
                    //     inst.deps,
                    // );
                    self.broadcast_try_pre_accept(&preply.instance, inst.ballot, inst.command.unwrap(), inst.seq, inst.deps);
                } else {
                    inst.from_leader.unwrap().borrow_mut().preparing = false;
                    self.start_phase_one(
                        &preply.instance,
                        inst.ballot,
                        inst.from_leader.unwrap().borrow().client_proposals.unwrap(),
                        ir.command,
                        ir.command.len() as u32,
                    );
                }
            }
            None => {
                let mut noop_deps: Vec<u32> = Vec::new();
                noop_deps[preply.instance.replica as usize] = preply.instance.slot - 1;
                inst.from_leader.unwrap().borrow_mut().preparing = false;
                epaxos_logic.instance_entry_space.insert(
                    preply.instance,
                    InstanceEntry {
                        ballot: inst.ballot,
                        command: None,
                        seq: 0,
                        deps: noop_deps,
                        instance: preply.instance,
                        state: Some(State::Accepted),
                        from_leader: inst.from_leader.clone(),
                    },
                );
                //self.send_accepts(preply.instance, inst.ballot, 0, 0, noop_deps);
                self.broadcast_accept(&preply.instance, inst.ballot, 0, 0, noop_deps);
            }
        }
    }

    fn handle_accept_reply(&self, accept_reply: &AcceptReplyPayload) {
        let mut epaxos_logic = self.epaxos_logic.lock().unwrap();
        let mut inst = epaxos_logic.instance_entry_space.get(&accept_reply.instance);

        if inst.unwrap().state == Some(State::Accepted) {
            // we've move on, these are delayed replies, so just ignore
            // XXX: Produce better?
            return;
        }

        if inst.unwrap().ballot != accept_reply.ballot {
            // XXX: Produce better?
            return;
        }

        if accept_reply.ok == 0 {
            // TODO: there is probably another active leader
            inst.unwrap().from_leader.unwrap().borrow_mut().nacks += 1;

            if accept_reply.ballot > inst.unwrap().from_leader.unwrap().borrow().max_recv_ballot {
                inst.unwrap().from_leader.unwrap().borrow().max_recv_ballot = accept_reply.ballot;
            }

            if inst.unwrap().from_leader.unwrap().borrow().nacks >= (epaxos_logic.info.n / 2).try_into().unwrap() {
                // TODO
            }
            return;
        }

        inst.unwrap().from_leader.unwrap().borrow_mut().accept_oks += 1;

        if inst.unwrap().from_leader.unwrap().borrow().accept_oks + 1 > (epaxos_logic.info.n / 2).try_into().unwrap() {
            epaxos_logic.instance_entry_space.get(&accept_reply.instance).unwrap().state = Some(State::Committed);
            epaxos_logic.update_committed(&accept_reply.instance);
            
            if !inst.unwrap().from_leader.unwrap().borrow().client_proposals.is_none() && epaxos_logic.info.dreply {
            //     // give clients the all clear
            // TODO: Redesign
			// for i := 0; i < len(inst.lb.clientProposals); i++ {
			// 	r.ReplyProposeTS(
			// 		&genericsmrproto.ProposeReplyTS{
			// 			TRUE,
			// 			inst.lb.clientProposals[i].CommandId,
			// 			state.NIL,
			// 			inst.lb.clientProposals[i].Timestamp},
			// 		inst.lb.clientProposals[i].Reply)
			// }
            }

            epaxos_logic.record_payload_metadata(inst.unwrap());
            epaxos_logic.sync();
            self.broadcast_commit(&accept_reply.instance, inst.unwrap().command.unwrap(), inst.unwrap().seq, inst.unwrap().deps);
        }
        


    }

    fn handle_pre_accept_reply(&self, paccrepliy: &PreAcceptReplyPayload) {
        
    }

    fn handle_try_pre_accept_reply(&self, try_pre_accept_reply: &TryPreAcceptReplyPayload) {

    }

    fn send_pre_accept_ok(&self, pre_accept_ok: PreAcceptOKPayload) {

    }

    // // send preaccept + handle preacceptreply
    // // broadcast && handle reply
    // fn send_pre_accepts(&self, payload: &Payload) -> Vec<Payload> {
    //     info!("Send_Pre_accepts");
    //     let mut pre_accept_oks = Vec::new();
    //     for replica_id in self.quorum_members.iter() {
    //         info!("Sending PreAccept to {:?}", replica_id);
    //         smol::block_on(async {
    //             let pre_accept_ok = self
    //                 .replicas
    //                 .get(replica_id)
    //                 .unwrap()
    //                 .pre_accept_async(&payload.to_grpc())
    //                 .unwrap_or_else(|e| {
    //                     panic!("Panic on unwrap pre_accept,{}", e);
    //                 });
    //             let value = pre_accept_ok.await;
    //             // handle preaccept reply && preacceptok 这里不仅调用handlereply 还有 handleok
    //             match value {
    //                 Err(e) => panic!("[PreAccept Stage] Replica panic,{:?}", e),
    //                 Ok(_payload) => {
    //                     // handle reply
    //                     pre_accept_oks.push(Payload::from_grpc(&_payload));
    //                 }
    //             }
    //         });
    //     }
    //     pre_accept_oks
    // }

    // fn send_accepts(&self, payload: &Payload) -> usize {
    //     info!("Send_accept!!");
    //     let mut accept_ok_count: usize = 1;
    //     for replica_id in self.quorum_members.iter() {
    //         smol::block_on(async {
    //             let accept_ok = self
    //                 .replicas
    //                 .get(replica_id)
    //                 .unwrap()
    //                 .accept_async(&payload.to_grpc());
    //             match accept_ok.unwrap().await {
    //                 Err(e) => panic!("[Paxos-Accept Stage] Replica panic {:?}", e),
    //                 Ok(_p) => {
    //                     // TODO: handle accept reply
    //                     accept_ok_count += 1;
    //                 }
    //             }
    //         });
    //     }
    //     accept_ok_count
    // }
    // fn send_commits(&self, payload: &Payload) {
    //     info!("Send commits");
    //     for replica_id in self.quorum_members.iter() {
    //         smol::block_on(async {
    //             let commit_ok = self
    //                 .replicas
    //                 .get(replica_id)
    //                 .unwrap()
    //                 .commit_async(&payload.to_grpc());
    //             info!("Sending Commit to replica {}", replica_id.0);
    //             let value = commit_ok.unwrap().await;
    //             match value {
    //                 Err(e) => error!("[Commit Stage] Replica panic {:?}", e),
    //                 Ok(_payload) => info!("Sending Commit to replica {}", replica_id.0),
    //             }
    //         });
    //     }
    // }

    // fn start_recovery_instance(&self, payload: &mut Payload) {
    //     let mut epaxos_logic = self.epaxos_logic.lock().unwrap();
    //     epaxos_logic._recovery_instance(payload);
    //     self.send_prepare(payload);
    // }

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

    // fn execute(&self) {
    //     info!("Executing");
    //     let epaxos_logic = self.epaxos_logic.lock().unwrap();
    //     //epaxos_logic.execute();
    // }
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
                &pre_accept_request.0.instance,
            );
            let mut state_change = State::PreAcceptedEq;
            if changed {
                state_change = State::PreAccepted;
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
                    if instance.state.unwrap() == State::Committed || instance.state.unwrap() == State::Executed{
                        // reordered handling of commit/accept and pre-accept
                        if instance.command.unwrap().len() == 0 {
                            epaxos_logic.instance_entry_space[&Instance {
                                replica: pre_accept_request.0.leader_id,
                                slot: pre_accept_request.0.instance.slot,
                            }]
                                .command = Some(pre_accept_request.0.command);
                            epaxos_logic.update_conflicts(
                                pre_accept_request.0.command,
                                &pre_accept_request.0.instance,
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
                        // TODO: make send specify replica
                        // self.send_pre_accept_reply(
                        //     pre_accept_request.0.leader_id,
                        //     PreAcceptReplyPayload {
                        //         instance: pre_accept_request.0.instance,
                        //         ok: 0,
                        //         ballot: instance.ballot,
                        //         command: instance.command,
                        //         seq: instance.seq,
                        //         deps: instance.deps,
                        //         commited_deps: epaxos_logic.commited_upto_instance,
                        //     },
                        // );
                    } else {
                        instance.command = Some(pre_accept_request.0.command);
                        instance.seq = seq;
                        instance.deps = deps;
                        instance.ballot = pre_accept_request.0.ballot;
                        instance.state = Some(state_change);
                    }
                }
                None => {
                    epaxos_logic.instance_entry_space.insert(
                        pre_accept_request.0.instance,
                        InstanceEntry {
                            ballot: pre_accept_request.0.ballot,
                            command: Some(pre_accept_request.0.command),
                            seq,
                            deps,
                            instance: pre_accept_request.0.instance,
                            state: Some(state_change),
                            from_leader: None,
                        },
                    );
                }
            }

            epaxos_logic.update_conflicts(
                pre_accept_request.0.command,
                &pre_accept_request.0.instance,
                pre_accept_request.0.seq,
            );
            epaxos_logic.record_payload_metadata(
                epaxos_logic
                    .instance_entry_space
                    .get(&pre_accept_request.0.instance)
                    .unwrap(),
            );
            epaxos_logic.record_commands(pre_accept_request.0.command);
            epaxos_logic.sync();

            // checkpoint && update latest check point info
            if pre_accept_request.0.command.len() == 0 {
                epaxos_logic.lastest_cp_replica = pre_accept_request.0.instance.replica;
                epaxos_logic.lastest_cp_instance = pre_accept_request.0.instance.slot;

                epaxos_logic.clear_hashtables();
            }

            if changed
                || uncommitted
                || pre_accept_request.0.instance.replica != pre_accept_request.0.leader_id
                || !epaxos_logic.is_initial_ballot(pre_accept_request.0.ballot)
            {
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
                // TODO: handle pre_accept_ok
                Ok(())
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
        info!("Received Accept");
        // let self_inner = Arc::<EpaxosServerInner>::clone(&self.inner);
        // let task = async move {
        //     let mut epaxos_logic = self_inner.epaxos_logic.lock().unwrap();
        //     let request = Accept(Payload::from_grpc(&req));
        //     let r = epaxos_logic.accept_(request);
        //     Ok(r.0.to_grpc())
        // };
        // super::util::spawn_grpc_task(sink, task);
        let self_inner = Arc::<EpaxosServerInner>::clone(&self.inner);
        let mut epaxos_logic = self_inner.epaxos_logic.lock().unwrap();
        let accept_request = Accept(AcceptPayload::from_grpc(&req));

        let mut inst = epaxos_logic.instance_entry_space.get(&Instance {
            replica: accept_request.0.leader_id,
            slot: accept_request.0.instance.slot,
        });
        if accept_request.0.seq > epaxos_logic.max_seq {
            epaxos_logic.max_seq = accept_request.0.seq + 1;
        }

        if !inst.is_none()
            && (inst.unwrap().state == Some(State::Committed) || inst.unwrap().state == Some(State::Executed))
        {
            // TODO: deal with better, return is not good
            return;
        }

        if accept_request.0.instance.slot
            > epaxos_logic.current_instance[accept_request.0.instance.replica as usize]
        {
            // update current_instance in the epaxos
            epaxos_logic.current_instance[accept_request.0.instance.replica as usize] =
                accept_request.0.instance.slot + 1;
        }

        match inst {
            Some(instance_entry) => {
                if accept_request.0.ballot < instance_entry.ballot {
                    // reply accept
                }
                instance_entry.state = Some(State::Accepted);
                instance_entry.seq = accept_request.0.seq;
                instance_entry.deps = accept_request.0.deps;
            }
            None => {
                epaxos_logic.instance_entry_space.insert(
                    Instance {
                        replica: accept_request.0.leader_id,
                        slot: accept_request.0.instance.slot,
                    },
                    InstanceEntry {
                        ballot: accept_request.0.ballot,
                        command: None,
                        seq: accept_request.0.seq,
                        deps: accept_request.0.deps,
                        instance: accept_request.0.instance,
                        state: Some(State::Accepted),
                        from_leader: None,
                    },
                );

                if accept_request.0.count == 0 {
                    //checkpoint
                    //update latest checkpoint info
                    epaxos_logic.lastest_cp_replica = accept_request.0.instance.replica;
                    epaxos_logic.lastest_cp_instance = accept_request.0.instance.slot;
                }
            }
        }
    }

    // handle commit
    fn commit(
        &mut self,
        ctx: ::grpcio::RpcContext,
        req: super::epaxos::CommitPayload,
        sink: ::grpcio::UnarySink<Empty>,
    ) {
        // info!("Commit");
        // let self_inner = Arc::<EpaxosServerInner>::clone(&self.inner);
        // let task = async move {
        //     let mut epaxos_logic = self_inner.epaxos_logic.lock().unwrap();
        //     let request = Commit(Payload::from_grpc(&req));
        //     epaxos_logic.commit_(request);
        //     let r = Empty::new();
        //     Ok(r)
        // };
        // super::util::spawn_grpc_task(sink, task);
    }

    fn prepare(
        &mut self,
        ctx: ::grpcio::RpcContext,
        req: super::epaxos::PreparePayload,
        sink: ::grpcio::UnarySink<super::epaxos::PrepareReplyPayload>,
    ) {
        let self_inner = Arc::<EpaxosServerInner>::clone(&self.inner);
        let mut epaxos_logic = self_inner.epaxos_logic.lock().unwrap();
        let prepare_request = Prepare(PreparePayload::from_grpc(&req));
        let mut inst = epaxos_logic
            .instance_entry_space
            .get(&prepare_request.0.instance);
        //let mut respond = PrepareReplyPayload::default();
        let mut nil_deps: Vec<u32> = Vec::new();

        match inst {
            None => {
                epaxos_logic.instance_entry_space.insert(
                    prepare_request.0.instance,
                    InstanceEntry {
                        ballot: prepare_request.0.ballot,
                        command: None,
                        seq: 0,
                        deps: nil_deps,
                        instance: prepare_request.0.instance,
                        state: None,
                        from_leader: Some(RefCell::new(CommandLeaderBookKeeping::default())),
                    },
                );

                let task = async {
                    let mut respond = PrepareReplyPayload {
                        accept_id: epaxos_logic.info.id.0,
                        ok: 1,
                        instance: prepare_request.0.instance,
                        ballot: u32::MAX, // -1
                        state: None,
                        command: Vec::new(),
                        seq: u32::MAX, // -1
                        deps: nil_deps,
                    };
                    Ok(respond.to_grpc())
                };

                crate::util::spawn_grpc_task(sink, task);
            }
            Some(mut instance) => {
                let mut ok = 1;
                if prepare_request.0.ballot < instance.ballot {
                    ok = 0;
                } else {
                    instance.ballot = prepare_request.0.ballot;
                }
                let task = async {
                    let mut respond = PrepareReplyPayload {
                        accept_id: epaxos_logic.info.id.0,
                        ok,
                        instance: prepare_request.0.instance,
                        ballot: instance.ballot,
                        state: instance.state.unwrap(),
                        command: instance.command.unwrap(),
                        seq: instance.seq,
                        deps: instance.deps,
                    };
                    Ok(respond.to_grpc())
                };

                // TODO: reply to prepare.leaderid
                // prepare_request.0.leader_id;
                crate::util::spawn_grpc_task(sink, task);
            }
        }
        // // 1. get most recent ballot number epoch.x.Y accepted for instance L.i
        // // 2. according to ballot return prepareok or nack.
        // let task = async move {
        //     let mut epaxos_logic = self_inner.epaxos_logic.lock().unwrap();
        //     let p = Payload::from_grpc(&req);
        //     match epaxos_logic.reply_prepare_ok(p) {
        //         Ok(_) => todo!(),
        //         Err(_) => todo!(),
        //     }
        // };
        // super::util::spawn_grpc_task(sink, task);
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
        sink: grpcio::UnarySink<crate::epaxos::Empty>,
    ) {
        //TODO： handle client retry

        //let mut epaxos_logic = self.epaxos_logic.lock().unwrap();
        let self_inner = Arc::<EpaxosServerInner>::clone(&self.inner);
        let mut epaxos_logic = self_inner.epaxos_logic.lock().unwrap();

        let queue = QUEUE_BUFFER.lock().unwrap();
        let propose_req = ProposePayload::from_grpc(&req);
        if queue.len() == 0 {
            let mut batch_size = MAXBATCH;
            queue.push_back(propose_req);
            // do something
        } else {
            if queue.len() > MAXBATCH {
                let mut batch_size = MAXBATCH;
            } else {
                let mut batch_size = queue.len();
            }
        }

        //let mut batch_size = epaxos_logic.propose_chan.len() + 1;
        // let mut batch_size = ctx.request_headers().len() + 1;
        // if batch_size > MAXBATCH {
        //     batch_size = MAXBATCH;
        // }

        let mut inst_no = epaxos_logic
            .current_instance
            .get(epaxos_logic.info.id.0 as usize)
            .unwrap();
        epaxos_logic.current_instance[epaxos_logic.info.id.0 as usize] += 1;

        info!("Starting instance {:?}", inst_no);
        //info!("Batching is {:?}", batch_size);

        // let mut cmds: Vec<Command> = Vec::with_capacity(batch_size as usize);
        // let mut proposals: Vec<ProposePayload> = Vec::with_capacity(batch_size as usize);
        // for i in 0..batch_size {
        //     let prop = epaxos_logic.propose_chan.pop();
        //     cmds.insert(i as usize, prop.command);
        //     proposals.insert(i as usize, prop);
        // }

        self_inner.start_phase_one(
            &Instance {
                replica: epaxos_logic.info.id.0,
                slot: *inst_no,
            },
            0,
            ProposePayload::from_grpc(&req),
            //batch_size,
        );

        let respond = Empty::new();
        sink.success(respond);
    }

    // handle try pre accept
    fn try_pre_accept(
        &mut self,
        ctx: grpcio::RpcContext,
        req: crate::epaxos::TryPreAcceptPayload,
        sink: grpcio::UnarySink<crate::epaxos::TryPreAcceptReplyPayload>,
    ) {
        let self_inner = Arc::<EpaxosServerInner>::clone(&self.inner);
        let mut epaxos_logic = self_inner.epaxos_logic.lock().unwrap();
        let mut try_pre_accept_request = TryPreAccept(TryPreAcceptPayload::from_grpc(&req));
        let mut inst = epaxos_logic
            .instance_entry_space
            .get(&try_pre_accept_request.0.instance);

        if !inst.is_none() && inst.unwrap().ballot > try_pre_accept_request.0.ballot {
            // ballot number too small
            let task = async {
                // TODO: lack leader id
                let try_pre_accept_reply = TryPreAcceptReplyPayload {
                    accept_id: epaxos_logic.info.id.0,
                    instance: try_pre_accept_request.0.instance,
                    ok: 0,
                    ballot: inst.unwrap().ballot,
                    conflict_instance: Some(try_pre_accept_request.0.instance),
                    conflict_state: Some(inst.unwrap().state.unwrap()), // TODO: decide how to design the state transmmit
                };
                Ok(try_pre_accept_reply.to_grpc())
            };
            // sink success
            crate::util::spawn_grpc_task(sink, task);
        }

        let (mut conflicts, mut conf_instance) = epaxos_logic.find_pre_accept_conflicts(
            try_pre_accept_request.0.command,
            try_pre_accept_request.0.instance,
            try_pre_accept_request.0.seq,
            try_pre_accept_request.0.deps,
        );
        if conflicts {
            let task = async {
                // TODO: lack leader id
                let try_pre_accept_reply = TryPreAcceptReplyPayload {
                    accept_id: epaxos_logic.info.id.0,
                    instance: try_pre_accept_request.0.instance,
                    ok: 0,
                    ballot: inst.unwrap().ballot,
                    conflict_instance: conf_instance,
                    conflict_state: Some(epaxos_logic
                        .instance_entry_space
                        .get(&conf_instance)
                        .unwrap()
                        .state), // 同上
                };
                Ok(try_pre_accept_reply.to_grpc())
            };
            crate::util::spawn_grpc_task(sink, task);
        } else {
            // can pre-accept
            if try_pre_accept_request.0.instance.slot
                >= epaxos_logic.current_instance[try_pre_accept_request.0.instance.replica as usize]
            {
                epaxos_logic.current_instance[try_pre_accept_request.0.instance.replica as usize] =
                    try_pre_accept_request.0.instance.slot + 1;
            }

            match inst {
                Some(instance_entry) => {
                    instance_entry.command = Some(try_pre_accept_request.0.command);
                    instance_entry.deps = try_pre_accept_request.0.deps;
                    instance_entry.seq = try_pre_accept_request.0.seq;
                    instance_entry.state = Some(State::PreAccepted);
                    instance_entry.ballot = try_pre_accept_request.0.ballot;
                }
                None => {
                    epaxos_logic.instance_entry_space.insert(
                        try_pre_accept_request.0.instance,
                        InstanceEntry {
                            ballot: try_pre_accept_request.0.ballot,
                            command: Some(try_pre_accept_request.0.command),
                            seq: try_pre_accept_request.0.seq,
                            deps: try_pre_accept_request.0.deps,
                            instance: try_pre_accept_request.0.instance,
                            state: Some(State::PreAccepted),
                            from_leader: None,
                        },
                    );
                }
            }

            let task = async {
                // TODO: lack leader id
                let try_pre_accept_reply = TryPreAcceptReplyPayload {
                    accept_id: epaxos_logic.info.id.0,
                    instance: try_pre_accept_request.0.instance,
                    ok: 1,
                    ballot: inst.unwrap().ballot,
                    conflict_instance: None,
                    conflict_state: None, // 同上
                };
                Ok(try_pre_accept_reply.to_grpc())
            };
            crate::util::spawn_grpc_task(sink, task);
        }
    }
}
