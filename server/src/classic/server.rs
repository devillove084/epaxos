#![allow(unused_variables)]
#![allow(incomplete_features)]
#![allow(unused_imports)]
#![allow(unused_mut)]
#![allow(unused_must_use)]
#![allow(unused_assignments)]

use std::collections::{HashMap, VecDeque};
use std::mem::MaybeUninit;
use std::ops::Add;

use std::panic;
use std::sync::{Arc, Mutex};

use crate::classic::config::{CHECKPOINT_PERIOD, DO_CHECKPOINTING, MAXBATCH};
use crate::classic::epaxos_info::{equal, EpaxosLogic};

use super::config::{REPLICAS_NUM, REPLICA_TEST};
use super::epaxos::Empty;
use super::epaxos_grpc::{EpaxosService, EpaxosServiceClient};
use super::message::*;
use crate::cli::cli::CLIParam;
use grpcio::{ChannelBuilder, Environment, UnarySink};
use log::{error, info};

lazy_static! {
    //static ref HOSTNAME: Mutex<String> = Mutex::new(String::new());
    pub static ref CP_COUNTER: Mutex<u32> = Mutex::new(0);
    pub static ref CP_MARKER: Vec<Command> = Vec::new();
    // Default queue buffer is vec
    // TODO: maybe better?
    pub static ref QUEUE_BUFFER: Mutex<VecDeque<ProposePayload>> = Mutex::new(VecDeque::new());
    pub static ref CONFLICT: Mutex<u32> = Mutex::new(0);
    pub static ref HAPPY: Mutex<u32> = Mutex::new(0);
    pub static ref SLOW: Mutex<u32> = Mutex::new(0);
    pub static ref WEIRD: Mutex<u32> = Mutex::new(0);
}

#[derive(Clone)]
pub struct EpaxosServerImpl {
    pub inner: Arc<EpaxosServerInner>,
}

pub struct EpaxosServerInner {
    pub store: Arc<Mutex<HashMap<String, i32>>>,
    pub epaxos_logic: Arc<Mutex<EpaxosLogic>>,
    pub replicas: HashMap<usize, EpaxosServiceClient>,
    pub quorum_members: Vec<u32>,
    pub max_recv_ballot: u32,
}

impl EpaxosServerImpl {
    pub fn init(id: u32, quorum_members: Vec<u32>, parm: CLIParam) -> Self {
        Self {
            inner: Arc::new(EpaxosServerInner::init(id, quorum_members, parm)),
        }
    }
}

impl EpaxosServerInner {
    fn init(id: u32, quorum_members: Vec<u32>, param: CLIParam) -> Self {
            let mut replicas = HashMap::new();
            info!("Initializing Replica {}", id);
            for i in 0..REPLICAS_NUM {
                if i != id as usize {
                    let env = Arc::new(Environment::new(1));
                    let ch = ChannelBuilder::new(env).connect(REPLICA_TEST[i]);
                    info!(">> Neighbor replica {} created", i);
                    let replica = EpaxosServiceClient::new(ch);
                    replicas.insert(i, replica);
                }
            }

            EpaxosServerInner {
                store: Arc::new(Mutex::new(HashMap::new())),
                epaxos_logic: Arc::new(Mutex::new(EpaxosLogic::init(param))),
                replicas: replicas,
                quorum_members: quorum_members,
                max_recv_ballot: 0,
            }
    }

    // pub async fn start_phase_one(
    //     &mut self,
    //     instance: &Instance,
    //     ballot: u32,
    //     proposal: ProposePayload,
    //     cmds: Vec<Command>,
    //     batch_size: u32,
    // ) {
    //     let mut epaxos_logic = self.epaxos_logic.lock().unwrap();
    //     //let cmds = proposal.command;
    //     let seq = 0;
    //     let deps: Vec<u32> = Vec::with_capacity(epaxos_logic.info.id.0 as usize);

    //     let (mut seq, mut deps, _) = epaxos_logic.update_attributes(cmds, seq, deps, instance);

    //     // TODO: Does is insert by current replica id and parm instance?
    //     epaxos_logic.instance_entry_space.insert(
    //         *instance,
    //         InstanceEntry {
    //             ballot,
    //             command: Some(proposal.command),
    //             seq,
    //             deps,
    //             instance: *instance,
    //             state: Some(State::PreAccepted),
    //             from_leader: Some(CommandLeaderBookKeeping {
    //                 client_proposals: Some(proposal),
    //                 max_recv_ballot: 0,
    //                 prepare_oks: 0,
    //                 all_equal: true,
    //                 pre_accept_oks: 0,
    //                 accept_oks: 0,
    //                 nacks: 0,
    //                 original_deps: deps.clone(),
    //                 commited_deps: vec![u32::MAX; deps.len()],
    //                 recovery_insts: None,
    //                 preparing: false,
    //                 trying_to_pre_accept: false,
    //                 possible_quorum: Vec::new(),
    //                 tpa_oks: 0,
    //                 commit_time: 0,
    //             }),
    //         },
    //     );

    //     epaxos_logic.update_conflicts(cmds, instance, seq);
    //     //epaxos_logic.update_conflicts(cmds, &instance, seq);

    //     if seq > epaxos_logic.max_seq {
    //         epaxos_logic.max_seq = seq + 1;
    //     }

    //     epaxos_logic
    //         .record_payload_metadata(epaxos_logic.instance_entry_space.get(instance).unwrap());
    //     epaxos_logic.record_commands(cmds);
    //     epaxos_logic.sync();

    //     //self.send_pre_accepts(instance, ballot, cmds, seq, deps);
    //     self.broadcast_pre_accepts(instance, ballot, cmds, seq, deps);

    //     CP_COUNTER.lock().unwrap().add_assign(batch_size);

    //     if epaxos_logic.info.id.0 == 0
    //         && DO_CHECKPOINTING
    //         && CP_COUNTER.lock().unwrap().ge(&CHECKPOINT_PERIOD)
    //     {
    //         CP_COUNTER.lock().unwrap().mul_assign(0);

    //         epaxos_logic.current_instance[epaxos_logic.info.id.0 as usize] += 1;
    //         instance.slot += 1;
    //         epaxos_logic.max_seq += 1;

    //         for q in 0..epaxos_logic.info.n {
    //             //deps.push(epaxos_logic.current_instance[q] );
    //             let slot = epaxos_logic.current_instance[q] - 1;
    //             deps.push(slot);
    //         }

    //         epaxos_logic.instance_entry_space.insert(
    //             *instance,
    //             InstanceEntry {
    //                 ballot: 0,
    //                 command: Some(CP_MARKER.to_vec()),
    //                 seq: epaxos_logic.max_seq,
    //                 deps,
    //                 instance: *instance,
    //                 state: Some(State::PreAccepted),
    //                 from_leader: Some(CommandLeaderBookKeeping {
    //                     max_recv_ballot: 0,
    //                     prepare_oks: 0,
    //                     all_equal: true,
    //                     pre_accept_oks: 0,
    //                     accept_oks: 0,
    //                     nacks: 0,
    //                     original_deps: deps,
    //                     commited_deps: Vec::new(),
    //                     recovery_insts: None,
    //                     preparing: false,
    //                     trying_to_pre_accept: false,
    //                     possible_quorum: Vec::new(),
    //                     tpa_oks: 0,
    //                     commit_time: 0,
    //                     client_proposals: None,
    //                 }),
    //             },
    //         );

    //         epaxos_logic.lastest_cp_instance = instance.slot;
    //         epaxos_logic.lastest_cp_replica = instance.replica;

    //         epaxos_logic.clear_hashtables();
    //         epaxos_logic
    //             .record_payload_metadata(epaxos_logic.instance_entry_space.get(instance).unwrap());
    //         epaxos_logic.record_commands(cmds);
    //         epaxos_logic.sync();

    //         self.broadcast_pre_accepts(instance, ballot, cmds, seq, deps);
    //         //self.send_pre_accepts(instance, ballot, cmds, seq, deps);
    //     }
    // }

    pub async fn broadcast_pre_accepts(
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
            leader_id: epaxos_logic.info.id,
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
        //let mut result = Vec::new();
        for replica_id in 0..n {
            if !epaxos_logic.info.alive[&epaxos_logic.info.preferred_peer_order[replica_id]] {
                continue;
            }

            smol::block_on(async {
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
                        self.handle_pre_accept_reply(&PreAcceptReplyPayload::from_grpc(
                            &pre_accept_reply_payload,
                        ));
                    }
                    Err(e) => {
                        error!("Error on received pre_accept_reply {:?}", &e);
                    }
                }
            });

            //result.push(task);
            info!("send pre accept payload");
        }

        // // XXX: Produce better?
        // for t in result {
        //     smol::block_on(async {
        //         t.await;
        //     });
        // }
    }

    pub async fn broadcast_prepare(&self, instance: &Instance, ballot: u32) {
        let mut epaxos_logic = self.epaxos_logic.lock().unwrap();
        let mut prepare_paylod = PreparePayload {
            leader_id: epaxos_logic.info.id,
            ballot,
            instance: *instance,
        };

        let mut n = epaxos_logic.info.n - 1;
        if epaxos_logic.info.thrifty {
            n = epaxos_logic.info.n / 2;
        }

        let mut q: usize = epaxos_logic.info.id as usize;
        let mut sent: usize = 0;
        //let mut result = Vec::new();
        while sent < n {
            q = (q + 1) % epaxos_logic.info.n;
            if q == epaxos_logic.info.id as usize {
                info!("Not enough replicas alive!");
            }

            if !epaxos_logic.info.alive[&(q as u32)] {
                continue;
            }

            // TODO: tryzip
            smol::block_on(async {
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
                        self.handle_prepare_reply(&PrepareReplyPayload::from_grpc(
                            &prepare_reply_payload,
                        ));
                    }
                    Err(error) => {
                        error!("Error when get the prepare reply payload{:?}", &error);
                    }
                }
            });

            //result.push(task);
            info!("Send Prepare payload and received prepare reply payloads");
            sent += 1;
        }

        // // XXX: Produce better?
        // for t in result {
        //     smol::block_on(async { t.await });
        // }
    }

    pub async fn broadcast_try_pre_accept(
        &self,
        instance: &Instance,
        ballot: u32,
        cmds: Vec<Command>,
        seq: u32,
        deps: Vec<u32>,
    ) {
        let mut epaxos_logic = self.epaxos_logic.lock().unwrap();
        let mut try_pre_accept_payload = TryPreAcceptPayload {
            leader_id: epaxos_logic.info.id,
            instance: *instance,
            ballot,
            command: cmds,
            seq,
            deps,
        };
        epaxos_logic
            .info
            .tpa_payload
            .as_ref()
            .map(|mut s| s = &try_pre_accept_payload);
        //let mut result = Vec::new();
        for q in 0..epaxos_logic.info.n {
            if q == epaxos_logic.info.id as usize {
                continue;
            }
            if !epaxos_logic.info.alive[&(q as u32)] {
                continue;
            }

            smol::block_on(async {
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
                        self.handle_try_pre_accept_reply(&TryPreAcceptReplyPayload::from_grpc(
                            &try_pre_accept_payload,
                        ))
                        .await;
                    }
                    Err(e) => {
                        error!("Error on receive try_pre_accept payload{:?}", &e);
                    }
                }
            });
            //result.push(task);
            info!("send try pre accept payload");
        }

        // for t in result {
        //     smol::block_on(async {
        //         t.await;
        //     });
        // }
    }

    pub async fn broadcast_accept(
        &self,
        instance: &Instance,
        ballot: u32,
        count: u32,
        seq: u32,
        deps: Vec<u32>,
    ) {
        let mut epaxos_logic = self.epaxos_logic.lock().unwrap();
        let accept_payload = AcceptPayload {
            leader_id: epaxos_logic.info.id,
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
                        self.handle_accept_reply(&AcceptReplyPayload::from_grpc(
                            &accept_reply_payload,
                        ));
                    }
                    Err(e) => {
                        error!("Error on receive accept reply payload");
                    }
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

    pub async fn broadcast_commit(
        &self,
        instance: &Instance,
        cmds: &Vec<Command>,
        seq: u32,
        deps: Vec<u32>,
    ) {
        let epaxos_logic = self.epaxos_logic.lock().unwrap();
        let commit_payload = CommitPayload {
            leader_id: epaxos_logic.info.id,
            instance: *instance,
            command: cmds.to_vec(),
            seq,
            deps: deps.clone(),
        };

        let commit_short_payload = CommitShortPayload {
            leader_id: epaxos_logic.info.id,
            instance: *instance,
            count: cmds.len() as u32,
            seq,
            deps: deps.clone(),
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
                        }
                        Err(e) => {
                            error!("Error on commit send, empty message miss");
                        }
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
                        }
                        Err(e) => {
                            error!("Error on commit_short send, empty message miss");
                        }
                    }
                });

                // result.push(task);
                // let sender = COMMIT_SHORT_RESULT_CHANNEL.1.clone();
                // sender.send(payload);
                info!("send commit_short payload");
            }
        }
    }

    pub async fn handle_prepare_reply(&self, preply: &PrepareReplyPayload) {
        let mut epaxos_logic = self.epaxos_logic.lock().unwrap();
        let nnn = epaxos_logic.info.n;
        if let Some(inst) = epaxos_logic.instance_entry_space.get_mut(&preply.instance) {
            //let mut epaxos_logic = self.epaxos_logic.lock().unwrap();
            let preparing = inst.from_leader.as_ref().unwrap().preparing;
            if inst.from_leader.is_none() || !preparing {
                // TODO: fix return
                // we've moved on -- these are delayed replies, so just ignore
                // TODO: should replies for non-current ballots be ignored?
                return;
            }

            if preply.ok == 0 {
                // TODO: there is probably another active leader, back off and retry later
                //inst.from_leader.as_ref().unwrap().borrow_mut().nacks += 1;
                inst.from_leader.as_mut().map(|s| s.nacks += 1);
            }

            if inst.from_leader.as_ref().unwrap().prepare_oks < nnn as u32 / 2 {
                // return or maybe record this as log
                return;
            }

            //Got an ACK (preply.OK == TRUE)

            inst.from_leader.as_mut().map(|mut s| s.prepare_oks += 1);
        }
        let mut inst = if let Some(inst) = epaxos_logic.instance_entry_space.get(&preply.instance) {
            inst.clone() // TODO:Deep Copy
        } else {
            return;
        };

        if inst.state.unwrap() == State::Committed || inst.state.unwrap() == State::Executed {
            epaxos_logic.instance_entry_space.insert(
                preply.instance,
                InstanceEntry {
                    ballot: inst.ballot,
                    command: Some(preply.command.clone()),
                    seq: preply.seq,
                    deps: preply.deps.clone(),
                    instance: preply.instance,
                    state: Some(State::Committed),
                    from_leader: None,
                },
            );

            //broadcastcommit
            self.broadcast_commit(
                &preply.instance,
                &inst.command.as_ref().unwrap(),
                preply.seq,
                preply.deps.clone(),
            ).await;
            return;
        }

        //let ri = inst.from_leader.as_mut().unwrap().recovery_insts.as_mut();
        if preply.state == Some(State::Accepted) {
            // if inst
            //     .from_leader
            //     .as_ref()
            //     .unwrap()
            //     .recovery_insts
            //     .as_ref()
            //     .is_none()
            if inst.from_leader.as_ref().map_or(false,|s| s.max_recv_ballot < preply.ballot)
            {
                inst.from_leader.as_mut().map(|mut s| {
                    s.recovery_insts.as_mut().map(|mut re| {
                        re.command = preply.command.clone();
                        re.state = preply.state.unwrap();
                        re.seq = preply.seq;
                        re.deps = preply.deps.clone();
                        re.pre_accept_count = 0;
                        re.command_leader_response = false;
                    });
                    s.max_recv_ballot = preply.ballot;
                });
            }
        }

        if (preply.state == Some(State::PreAccepted) && preply.state == Some(State::PreAcceptedEq))
            && (inst
                .from_leader
                .as_ref()
                .unwrap()
                .recovery_insts
                .as_ref()
                .is_none()
                || inst
                    .from_leader
                    .as_ref()
                    .unwrap()
                    .recovery_insts
                    .as_ref()
                    .unwrap()
                    .state
                    < State::Accepted)
        {
            if inst
                .from_leader
                .as_mut()
                .unwrap()
                .recovery_insts
                .as_mut()
                .is_none()
            {
                inst.from_leader
                    .as_mut()
                    .unwrap()
                    .recovery_insts
                    .as_mut()
                    .map(|s| {
                        s.command = preply.command.clone();
                    });
            } else if preply.seq == inst.seq && equal(preply.deps.clone(), inst.deps.clone()) {
                inst.from_leader
                    .as_mut()
                    .unwrap()
                    .recovery_insts
                    .as_mut()
                    .map(|s| {
                        s.pre_accept_count += 1;
                    });
            } else if preply.state == Some(State::PreAccepted) {
                inst.from_leader
                    .as_mut()
                    .unwrap()
                    .recovery_insts
                    .as_mut()
                    .map(|s| {
                        s.command = preply.command.clone();
                        s.state = preply.state.unwrap();
                        s.seq = preply.seq;
                        s.deps = preply.deps.clone();
                        s.pre_accept_count += 1;
                        s.command_leader_response = false;
                    });
            } else if preply.accept_id == preply.instance.replica {
                inst.from_leader
                    .as_mut()
                    .unwrap()
                    .recovery_insts
                    .as_mut()
                    .map(|s| {
                        s.command_leader_response = true;
                    });
            }
        }

        if inst.from_leader.as_ref().unwrap().recovery_insts.is_none() {
            //at least one replica has (pre-)accepted this instance
            if inst
                .from_leader
                .as_ref()
                .unwrap()
                .recovery_insts
                .as_ref()
                .unwrap()
                .state
                == State::Accepted
                || (!inst
                    .from_leader
                    .as_ref()
                    .unwrap()
                    .recovery_insts
                    .as_ref()
                    .unwrap()
                    .command_leader_response
                    && inst
                        .from_leader
                        .as_ref()
                        .unwrap()
                        .recovery_insts
                        .as_ref()
                        .unwrap()
                        .pre_accept_count
                        >= epaxos_logic.info.n as u32 / 2
                    && (epaxos_logic.info.thrifty
                        || inst
                            .from_leader
                            .as_ref()
                            .unwrap()
                            .recovery_insts
                            .as_ref()
                            .unwrap()
                            .state
                            == State::PreAcceptedEq))
            {
                // safe to go to accept phase
                inst.command = Some(
                    inst.from_leader
                        .as_ref()
                        .unwrap()
                        .recovery_insts
                        .as_ref()
                        .unwrap()
                        .command
                        .clone(),
                );
                inst.seq = inst
                    .from_leader
                    .as_ref()
                    .unwrap()
                    .recovery_insts
                    .as_ref()
                    .unwrap()
                    .seq;
                inst.deps = inst
                    .from_leader
                    .as_ref()
                    .unwrap()
                    .recovery_insts
                    .as_ref()
                    .unwrap()
                    .deps
                    .clone();
                inst.from_leader.as_mut().map(|s| {
                    s.preparing = false;
                });

                // broadcast accepts
                self.broadcast_accept(
                    &preply.instance,
                    inst.ballot,
                    inst.command.as_ref().unwrap().len() as u32,
                    inst.seq,
                    inst.deps.clone(),
                ).await;
            } else if !inst
                .from_leader
                .as_ref()
                .unwrap()
                .recovery_insts
                .as_ref()
                .unwrap()
                .command_leader_response
                && inst
                    .from_leader
                    .as_ref()
                    .unwrap()
                    .recovery_insts
                    .as_ref()
                    .unwrap()
                    .pre_accept_count
                    >= (epaxos_logic.info.n as u32 / 2 + 1) / 2
            {
                //send TryPreAccepts
                //but first try to pre-accept on the local replica
                inst.from_leader.as_mut().map(|s| {
                    s.pre_accept_oks = 0;
                    s.nacks = 0;
                    s.possible_quorum.clear();
                });

                for q in 0..epaxos_logic.info.n {
                    inst.from_leader.as_mut().map(|s| {
                        s.possible_quorum[q] = true;
                    });
                }

                let (conflict, replica_q, instance_i) = epaxos_logic.find_pre_accept_conflicts(
                    &inst
                        .from_leader
                        .as_ref()
                        .unwrap()
                        .recovery_insts
                        .as_ref()
                        .unwrap()
                        .command
                        .clone(),
                    preply.instance,
                    inst.from_leader
                        .as_ref()
                        .unwrap()
                        .recovery_insts
                        .as_ref()
                        .unwrap()
                        .seq,
                    &inst.from_leader
                        .as_ref()
                        .unwrap()
                        .recovery_insts
                        .as_ref()
                        .unwrap()
                        .deps,
                );
                if conflict {
                    if epaxos_logic
                        .instance_entry_space
                        .get(&Instance {
                            replica: replica_q,
                            slot: instance_i,
                        })
                        .unwrap()
                        .state
                        .unwrap()
                        == State::Committed
                    {
                        // start Phase1 in the initial leader's instance
                        // self.start_phase_one(
                        //     &preply.instance,
                        //     inst.ballot,
                        //     inst.from_leader.map(|s| {s.client_proposals}).unwrap().unwrap(),
                        //     ir.command,
                        //     ir.command.len() as u32,
                        // );
                        // return;
                    } else {
                        inst.from_leader.as_mut().map(|s| s.nacks = 1);
                        inst.from_leader
                            .as_mut()
                            .map(|s| s.possible_quorum[epaxos_logic.info.id as usize] = false);
                    }
                } else {
                    inst.command = Some(
                        inst.from_leader
                            .as_ref()
                            .unwrap()
                            .recovery_insts
                            .as_ref()
                            .unwrap()
                            .command
                            .clone(),
                    );
                    inst.seq = inst
                        .from_leader
                        .as_ref()
                        .unwrap()
                        .recovery_insts
                        .as_ref()
                        .unwrap()
                        .seq;
                    inst.deps = inst
                        .from_leader
                        .as_ref()
                        .unwrap()
                        .recovery_insts
                        .as_ref()
                        .unwrap()
                        .deps
                        .clone();
                    inst.state = Some(State::PreAccepted);
                    inst.from_leader.as_mut().map(|s| {
                        s.pre_accept_oks = 1;
                    });
                }

                inst.from_leader.as_mut().map(|s| s.preparing = false);
                inst.from_leader
                    .as_mut()
                    .map(|s| s.trying_to_pre_accept = true);
                self.broadcast_try_pre_accept(
                    &preply.instance,
                    inst.ballot,
                    inst.command.as_ref().unwrap().to_vec(),
                    inst.seq,
                    inst.deps.clone(),
                );
            } else {
                inst.from_leader.as_mut().map(|s| s.preparing = false);
                // self.start_phase_one(
                //     &preply.instance,
                //     inst.ballot,
                //     inst.from_leader.map(|s| {s.client_proposals}).unwrap().unwrap(),
                //     ir.command,
                //     ir.command.len() as u32,
                // );
            }
        } else {
            let mut noop_deps: Vec<u32> = Vec::new();
            noop_deps[preply.instance.replica as usize] = preply.instance.slot - 1;
            //inst.from_leader.unwrap().borrow_mut().preparing = false;
            inst.from_leader.as_mut().map(|s| s.preparing = false);
            epaxos_logic.instance_entry_space.insert(
                preply.instance,
                InstanceEntry {
                    ballot: inst.ballot,
                    command: None,
                    seq: 0,
                    deps: noop_deps.clone(),
                    instance: preply.instance,
                    state: Some(State::Accepted),
                    from_leader: inst.from_leader.clone(),
                },
            );
            self.broadcast_accept(&preply.instance, inst.ballot, 0, 0, noop_deps.clone());
        }
    }
    //epaxos_logic._handle_prepare_reply(preply, self);

    pub async fn handle_accept_reply(&self, accept_reply: &AcceptReplyPayload) {
        let mut epaxos_logic = self.epaxos_logic.lock().unwrap();
        if let Some(inst) = epaxos_logic
            .instance_entry_space
            .get_mut(&accept_reply.instance)
        {
            if inst.state == Some(State::Accepted) {
                // TODO: we've move on, these are delayed replies, so just ignore
                return;
            }
            if inst.ballot != accept_reply.ballot {
                return;
            }

            let mut epaxos_logic = self.epaxos_logic.lock().unwrap();
            if accept_reply.ok == 0 {
                // TODO: there is probably another active leader
                inst.from_leader.as_mut().map(|mut s| s.nacks += 1);

                if accept_reply.ballot > inst.from_leader.as_ref().unwrap().max_recv_ballot {
                    inst.from_leader
                        .as_mut()
                        .map(|mut s| s.max_recv_ballot = accept_reply.ballot);
                }

                if inst.from_leader.as_ref().unwrap().nacks >= epaxos_logic.info.n as u32 / 2 {
                    // TODO
                }
                return;
            }

            inst.from_leader.as_mut().map(|mut s| s.accept_oks += 1);
            //inst.unwrap().from_leader.unwrap().borrow_mut().accept_oks += 1;
            let aok = inst.from_leader.as_ref().unwrap().accept_oks;
            if aok + 1 > epaxos_logic.info.n as u32 / 2 {
                epaxos_logic
                    .instance_entry_space
                    .get_mut(&accept_reply.instance)
                    .as_mut()
                    .map(|s| s.state = Some(State::Committed));
                epaxos_logic.update_committed(&accept_reply.instance);

                let client_proposals = inst.from_leader.as_ref().unwrap().client_proposals.as_ref();
                if client_proposals.is_none() && epaxos_logic.info.dreply {
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

                epaxos_logic.record_payload_metadata(inst);
                epaxos_logic.sync();
                self.broadcast_commit(
                    &accept_reply.instance,
                    &inst.command.as_ref().unwrap(),
                    inst.seq,
                    inst.deps.clone(),
                );
            }
        }
        // if inst.state == Some(State::Accepted) {
        //     // TODO: we've move on, these are delayed replies, so just ignore
        //     return;
        // }
        // if inst.ballot != accept_reply.ballot {
        //     return;
        // }
        // if accept_reply.ok == 0 {
        //     // TODO: there is probably another active leader
        //     inst.from_leader.map(|mut s| s.nacks += 1);

        //     if accept_reply.ballot > inst.from_leader.map(|s| s.max_recv_ballot).unwrap() {
        //         inst.from_leader
        //             .map(|mut s| s.max_recv_ballot = accept_reply.ballot);
        //     }

        //     if inst.from_leader.map(|s| s.nacks).unwrap() >= epaxos_logic.info.n as u32 / 2 {
        //         // TODO
        //     }
        //     return;
        // }

        // inst.from_leader.map(|mut s| s.accept_oks += 1);
        // //inst.unwrap().from_leader.unwrap().borrow_mut().accept_oks += 1;
        // let aok = inst.from_leader.map(|mut s| s.accept_oks);
        // if aok.unwrap() + 1 > epaxos_logic.info.n as u32 / 2 {
        //     epaxos_logic
        //         .instance_entry_space
        //         .get(&accept_reply.instance)
        //         .map(|mut s| s.state = Some(State::Committed));
        //     epaxos_logic.update_committed(&accept_reply.instance);

        //     let client_proposals = inst.from_leader.map(|s| s.client_proposals).unwrap();
        //     if client_proposals.is_none() && epaxos_logic.info.dreply {
        //         //     // give clients the all clear
        //         // TODO: Redesign
        //         // for i := 0; i < len(inst.lb.clientProposals); i++ {
        //         // 	r.ReplyProposeTS(
        //         // 		&genericsmrproto.ProposeReplyTS{
        //         // 			TRUE,
        //         // 			inst.lb.clientProposals[i].CommandId,
        //         // 			state.NIL,
        //         // 			inst.lb.clientProposals[i].Timestamp},
        //         // 		inst.lb.clientProposals[i].Reply)
        //         // }
        //     }

        //     epaxos_logic.record_payload_metadata(inst);
        //     epaxos_logic.sync();
        //     self.broadcast_commit(
        //         &accept_reply.instance,
        //         &inst.command.unwrap(),
        //         inst.seq,
        //         inst.deps,
        //     );
        // }
    }

    pub async fn handle_pre_accept_reply(&self, paccreply: &PreAcceptReplyPayload) {
        info!("Handling PreAccept reply");
        let mut epaxos_logic = self.epaxos_logic.lock().unwrap();
        if let Some(inst) = epaxos_logic
            .instance_entry_space
            .get_mut(&paccreply.instance)
        {
            if inst.state.unwrap() == State::PreAccepted {
                // we've moved on, this is a delayed reply
                return;
            }

            if inst.ballot != paccreply.ballot {
                return;
            }

            // XXX: Now? again?
            let mut epaxos_logic = self.epaxos_logic.lock().unwrap();
            if paccreply.ok == 0 {
                // TODO: there is probably another active leader
                inst.from_leader.as_mut().map(|mut s| s.nacks += 1);
                if paccreply.ballot > inst.from_leader.as_ref().unwrap().max_recv_ballot {
                    //inst.from_leader.unwrap().borrow_mut().max_recv_ballot = paccreply.ballot;
                    inst.from_leader
                        .as_mut()
                        .map(|s| s.max_recv_ballot = paccreply.ballot);
                }

                if inst.from_leader.as_ref().unwrap().nacks >= epaxos_logic.info.n as u32 / 2 {
                    // TODO
                }

                return;
            }

            inst.from_leader.as_mut().map(|s| s.pre_accept_oks += 1);

            //let mut epual = bool::default();
            let (newseq, newdeps, equal) = epaxos_logic.merge_attributes(
                inst.seq,
                inst.deps.clone(),
                paccreply.seq,
                paccreply.deps.clone(),
            );
            inst.seq = newseq;
            inst.deps = newdeps;

            // XXX: 3? maybe not a specity number
            if (epaxos_logic.info.n <= 3 && !epaxos_logic.info.thrifty)
                || inst.from_leader.as_ref().unwrap().pre_accept_oks > 1
            {
                //inst.from_leader.unwrap().borrow_mut().all_equal = inst.from_leader.unwrap().borrow().all_equal && epual;
                inst.from_leader
                    .as_mut()
                    .map(|mut s| s.all_equal = s.all_equal && equal);
                if !equal {
                    // TODO: Put this global var in the
                    CONFLICT.lock().unwrap().add(1);
                }
            }

            let mut all_commited = true;
            for q in 0..epaxos_logic.info.n {
                if inst.from_leader.as_ref().unwrap().commited_deps[q] < paccreply.commited_deps[q]
                {
                    inst.from_leader
                        .as_mut()
                        .map(|s| s.commited_deps[q] = paccreply.commited_deps[q]);
                }
                if inst.from_leader.as_ref().unwrap().commited_deps[q]
                    < epaxos_logic.commited_upto_instance[q]
                {
                    inst.from_leader
                        .as_mut()
                        .map(|s| s.commited_deps[q] = epaxos_logic.commited_upto_instance[q]);
                }
                if inst.from_leader.as_ref().unwrap().commited_deps[q] < inst.deps[q] {
                    all_commited = false;
                }
            }

            if inst.from_leader.as_ref().unwrap().pre_accept_oks >= epaxos_logic.info.n as u32 / 2
                && inst.from_leader.as_ref().unwrap().all_equal
                && all_commited
                && epaxos_logic.is_initial_ballot(inst.ballot)
            {
                HAPPY.lock().unwrap().add(1);
                info!("fast path in instance{:?}", paccreply.instance);
                //epaxos_logic.instance_entry_space.get_mut(&paccreply.instance).unwrap().state = Some(State::Committed);
                epaxos_logic
                    .instance_entry_space
                    .get_mut(&paccreply.instance)
                    .map(|mut s| s.state = Some(State::Committed));
                epaxos_logic.update_committed(&paccreply.instance);
                if !inst
                    .from_leader
                    .as_ref()
                    .unwrap()
                    .client_proposals
                    .is_none()
                    && epaxos_logic.info.dreply
                {
                    // TODO: Fix me
                    //     // give clients the all clear
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

                epaxos_logic.record_payload_metadata(inst);
                epaxos_logic.sync();
                self.broadcast_commit(
                    &paccreply.instance,
                    &inst.command.as_ref().unwrap(),
                    inst.seq,
                    inst.deps.clone(),
                );
            } else if inst.from_leader.as_ref().unwrap().pre_accept_oks
                >= epaxos_logic.info.n as u32 / 2
            {
                if !all_commited {
                    WEIRD.lock().unwrap().add(1);
                }
                SLOW.lock().unwrap().add(1);
                inst.state = Some(State::Accepted);
                self.broadcast_accept(
                    &paccreply.instance,
                    inst.ballot,
                    inst.command.as_ref().unwrap().len() as u32,
                    inst.seq,
                    inst.deps.clone(),
                );
            }
        }

        // if inst.state.unwrap() == State::PreAccepted {
        //     // we've moved on, this is a delayed reply
        //     return;
        // }

        // if inst.ballot != paccreply.ballot {
        //     return;
        // }

        // if paccreply.ok == 0 {
        //     // TODO: there is probably another active leader
        //     inst.from_leader.map(|mut s| s.nacks += 1);
        //     if paccreply.ballot > inst.from_leader.unwrap().max_recv_ballot {
        //         //inst.from_leader.unwrap().borrow_mut().max_recv_ballot = paccreply.ballot;
        //         inst.from_leader
        //             .map(|mut s| s.max_recv_ballot = paccreply.ballot);
        //     }

        //     if inst.from_leader.unwrap().nacks >= epaxos_logic.info.n as u32 / 2 {
        //         // TODO
        //     }

        //     return;
        // }

        // inst.from_leader.map(|mut s| s.pre_accept_oks += 1);

        // let mut epual = bool::default();
        // let (newseq, newdeps, equal) =
        //     epaxos_logic.merge_attributes(inst.seq, inst.deps, paccreply.seq, paccreply.deps);
        // inst.seq = newseq;
        // inst.deps = newdeps;

        // // XXX: 3? maybe not a specity number
        // if (epaxos_logic.info.n <= 3 && !epaxos_logic.info.thrifty)
        //     || inst.from_leader.unwrap().pre_accept_oks > 1
        // {
        //     //inst.from_leader.unwrap().borrow_mut().all_equal = inst.from_leader.unwrap().borrow().all_equal && epual;
        //     inst.from_leader
        //         .map(|mut s| s.all_equal = s.all_equal && equal);
        //     if !epual {
        //         // TODO: Put this global var in the
        //         CONFLICT.lock().unwrap().add(1);
        //     }
        // }

        // let mut all_commited = true;
        // for q in 0..epaxos_logic.info.n {
        //     if inst.from_leader.unwrap().commited_deps[q] < paccreply.commited_deps[q] {
        //         inst.from_leader
        //             .map(|mut s| s.commited_deps[q] = paccreply.commited_deps[q]);
        //     }
        //     if inst.from_leader.unwrap().commited_deps[q] < epaxos_logic.commited_upto_instance[q] {
        //         inst.from_leader
        //             .map(|mut s| s.commited_deps[q] = epaxos_logic.commited_upto_instance[q]);
        //     }
        //     if inst.from_leader.unwrap().commited_deps[q] < inst.deps[q] {
        //         all_commited = false;
        //     }
        // }

        // if inst.from_leader.unwrap().pre_accept_oks >= epaxos_logic.info.n as u32 / 2
        //     && inst.from_leader.unwrap().all_equal
        //     && all_commited
        //     && epaxos_logic.is_initial_ballot(inst.ballot)
        // {
        //     HAPPY.lock().unwrap().add(1);
        //     info!("fast path in instance{:?}", paccreply.instance);
        //     //epaxos_logic.instance_entry_space.get_mut(&paccreply.instance).unwrap().state = Some(State::Committed);
        //     epaxos_logic
        //         .instance_entry_space
        //         .get_mut(&paccreply.instance)
        //         .map(|mut s| s.state = Some(State::Committed));
        //     epaxos_logic.update_committed(&paccreply.instance);
        //     if !inst.from_leader.unwrap().client_proposals.is_none() && epaxos_logic.info.dreply {
        //         // TODO: Fix me
        //         //     // give clients the all clear
        //         // for i := 0; i < len(inst.lb.clientProposals); i++ {
        //         // 	r.ReplyProposeTS(
        //         // 		&genericsmrproto.ProposeReplyTS{
        //         // 			TRUE,
        //         // 			inst.lb.clientProposals[i].CommandId,
        //         // 			state.NIL,
        //         // 			inst.lb.clientProposals[i].Timestamp},
        //         // 		inst.lb.clientProposals[i].Reply)
        //         // }
        //     }

        //     epaxos_logic.record_payload_metadata(inst);
        //     epaxos_logic.sync();
        //     self.broadcast_commit(
        //         &paccreply.instance,
        //         &inst.command.unwrap(),
        //         inst.seq,
        //         inst.deps,
        //     );
        // } else if inst.from_leader.unwrap().pre_accept_oks >= epaxos_logic.info.n as u32 / 2 {
        //     if !all_commited {
        //         WEIRD.lock().unwrap().add(1);
        //     }
        //     SLOW.lock().unwrap().add(1);
        //     inst.state = Some(State::Accepted);
        //     self.broadcast_accept(
        //         &paccreply.instance,
        //         inst.ballot,
        //         inst.command.unwrap().len() as u32,
        //         inst.seq,
        //         inst.deps,
        //     );
        // }
    }

    pub async fn handle_try_pre_accept_reply(
        &self,
        try_pre_accept_reply: &TryPreAcceptReplyPayload,
    ) {
        let mut epaxos_logic = self.epaxos_logic.lock().unwrap();
        if let Some(inst) = epaxos_logic
            .instance_entry_space
            .get_mut(&try_pre_accept_reply.instance)
        {
            if inst.from_leader.is_none()
                || !inst.from_leader.as_ref().unwrap().trying_to_pre_accept
                || inst.from_leader.as_ref().unwrap().recovery_insts.is_none()
            {
                return;
            }
            // XXX: Why again?
            let mut epaxos_logic = self.epaxos_logic.lock().unwrap();
            if try_pre_accept_reply.ok > 0 {
                inst.from_leader.as_mut().map(|s| s.pre_accept_oks += 1);
                inst.from_leader.as_mut().map(|s| s.tpa_oks += 1);
                if inst.from_leader.as_ref().unwrap().pre_accept_oks
                    >= epaxos_logic.info.n as u32 / 2
                {
                    //it's safe to start Accept phase
                    inst.command = Some(
                        inst.from_leader
                            .as_ref()
                            .unwrap()
                            .recovery_insts
                            .as_ref()
                            .unwrap()
                            .command
                            .clone(),
                    );
                    inst.seq = inst
                        .from_leader
                        .as_ref()
                        .unwrap()
                        .recovery_insts
                        .as_ref()
                        .unwrap()
                        .seq;
                    inst.deps = inst
                        .from_leader
                        .as_ref()
                        .unwrap()
                        .recovery_insts
                        .as_ref()
                        .unwrap()
                        .deps
                        .clone();
                    inst.state = Some(State::Accepted);
                    inst.from_leader
                        .as_mut()
                        .map(|s| s.trying_to_pre_accept = false);
                    inst.from_leader.as_mut().map(|s| s.accept_oks = 0);
                    self.broadcast_accept(
                        &try_pre_accept_reply.instance,
                        inst.ballot,
                        inst.command.as_ref().unwrap().len() as u32,
                        inst.seq,
                        inst.deps.clone(),
                    );
                }
            } else {
                inst.from_leader.as_mut().map(|s| s.nacks += 1);
                //inst.unwrap().from_leader.unwrap().borrow_mut().nacks += 1;
                if try_pre_accept_reply.ballot > inst.ballot {
                    //TODO: retry with higher ballot
                    return;
                }
                inst.from_leader.as_mut().map(|s| s.tpa_oks += 1);
                //inst.from_leader.unwrap().borrow_mut().tpa_oks += 1;
                if try_pre_accept_reply.conflict_instance.unwrap() == try_pre_accept_reply.instance
                {
                    //TODO: re-run prepare
                    //inst.unwrap().from_leader.unwrap().borrow_mut().trying_to_pre_accept = false;
                    inst.from_leader
                        .as_mut()
                        .map(|s| s.trying_to_pre_accept = false);
                    return;
                }

                inst.from_leader
                    .as_mut()
                    .map(|s| s.possible_quorum[try_pre_accept_reply.accept_id as usize] = false);
                inst.from_leader.as_mut().map(|s| {
                    s.possible_quorum
                        [try_pre_accept_reply.conflict_instance.unwrap().slot as usize] = false
                });

                let mut not_in_quorum = 0;
                for q in 0..epaxos_logic.info.n {
                    if !inst.from_leader.as_ref().unwrap().possible_quorum
                        [try_pre_accept_reply.accept_id as usize]
                    {
                        not_in_quorum += 1;
                    }
                }

                if try_pre_accept_reply.conflict_state.unwrap() >= State::Committed
                    || not_in_quorum > epaxos_logic.info.n as u32 / 2
                {
                    //abandon recovery, restart from phase 1
                    //inst.unwrap().from_leader.unwrap().borrow_mut().trying_to_pre_accept = false;
                    inst.from_leader
                        .as_mut()
                        .map(|mut s| s.trying_to_pre_accept = false);
                    // self.start_phase_one(
                    //     &try_pre_accept_reply.instance,
                    //     inst.ballot,
                    //     inst.from_leader.unwrap().client_proposals.unwrap(),
                    //     ir.unwrap().command,
                    //     ir.unwrap().command.len() as u32,
                    // );
                }

                if not_in_quorum == epaxos_logic.info.n as u32 / 2 {
                    //this is to prevent defer cycles
                    let (present, dp, _) = epaxos_logic.deferred_by_instance(
                        try_pre_accept_reply.instance.replica,
                        try_pre_accept_reply.instance.slot,
                    );
                    if present {
                        if inst.from_leader.as_ref().unwrap().possible_quorum[dp as usize] {
                            //an instance whose leader must have been in this instance's quorum has been deferred for this instance => contradiction
                            //abandon recovery, restart from phase 1
                            // inst.unwrap().from_leader.unwrap().borrow_mut().trying_to_pre_accept = false;
                            inst.from_leader
                                .as_mut()
                                .map(|mut s| s.trying_to_pre_accept = false);
                            // self.start_phase_one(
                            //     &try_pre_accept_reply.instance,
                            //     inst.ballot,
                            //     inst.from_leader.unwrap().client_proposals.unwrap(),
                            //     ir.unwrap().command,
                            //     ir.unwrap().command.len() as u32,
                            // );
                        }
                    }
                }

                if inst.from_leader.as_ref().unwrap().tpa_oks >= epaxos_logic.info.n as u32 / 2 {
                    //defer recovery and update deferred information
                    epaxos_logic.update_deferred(
                        try_pre_accept_reply.instance.replica,
                        try_pre_accept_reply.instance.slot,
                        try_pre_accept_reply.conflict_instance.unwrap().replica,
                        try_pre_accept_reply.conflict_instance.unwrap().slot,
                    );
                    inst.from_leader
                        .as_mut()
                        .map(|mut s| s.trying_to_pre_accept = false);
                }
            }
        }
        //     } else if inst.is_none()
        //     || inst.as_ref().unwrap().from_leader.is_none()
        //     || !inst.as_ref().unwrap().from_leader.as_ref().unwrap().trying_to_pre_accept
        //     || inst.as_ref().unwrap().from_leader.as_ref().unwrap().recovery_insts.is_none()
        // {
        //     return;
        // }

        //let inst = inst.unwrap();
        //let ir = inst.as_ref().unwrap().from_leader.as_ref().unwrap().recovery_insts.as_ref();
        // if try_pre_accept_reply.ok > 0 {
        //     inst.as_ref().unwrap().from_leader.as_ref().map(|s| s.pre_accept_oks += 1);
        //     inst.as_ref().unwrap().from_leader.as_ref().map(|s| s.tpa_oks += 1);
        //     if inst.as_ref().unwrap().from_leader.as_ref().unwrap().pre_accept_oks >= epaxos_logic.info.n as u32 / 2 {
        //         //it's safe to start Accept phase
        //         inst.unwrap().command = Some(inst.as_ref().unwrap().from_leader.as_ref().unwrap().recovery_insts.as_ref().unwrap().command.clone());
        //         inst.unwrap().seq =  inst.as_ref().unwrap().from_leader.as_ref().unwrap().recovery_insts.as_ref().unwrap().seq;
        //         inst.unwrap().deps =  inst.as_ref().unwrap().from_leader.as_ref().unwrap().recovery_insts.as_ref().unwrap().deps.clone();
        //         inst.unwrap().state = Some(State::Accepted);
        //         inst.unwrap().from_leader.map(|mut s| s.trying_to_pre_accept = false);
        //         inst.unwrap().from_leader.map(|mut s| s.accept_oks = 0);
        //         self.broadcast_accept(
        //             &try_pre_accept_reply.instance,
        //             inst.unwrap().ballot,
        //             inst.unwrap().command.as_ref().unwrap().len() as u32,
        //             inst.unwrap().seq,
        //             inst.unwrap().deps.clone(),
        //         );
        //     }
        // } else {
        //     inst.as_ref().unwrap().from_leader.as_ref().map(|mut s| s.nacks += 1);
        //     //inst.unwrap().from_leader.unwrap().borrow_mut().nacks += 1;
        //     if try_pre_accept_reply.ballot > inst.unwrap().ballot {
        //         //TODO: retry with higher ballot
        //         return;
        //     }
        //     inst.unwrap().from_leader.as_mut().map(|s| s.tpa_oks += 1);
        //     //inst.from_leader.unwrap().borrow_mut().tpa_oks += 1;
        //     if try_pre_accept_reply.conflict_instance.unwrap() == try_pre_accept_reply.instance {
        //         //TODO: re-run prepare
        //         //inst.unwrap().from_leader.unwrap().borrow_mut().trying_to_pre_accept = false;
        //         inst.unwrap().from_leader.as_mut().map(|s| s.trying_to_pre_accept = false);
        //         return;
        //     }

        //     inst.unwrap().from_leader.as_mut()
        //         .map(|s| s.possible_quorum[try_pre_accept_reply.accept_id as usize] = false);
        //     inst.unwrap().from_leader.as_mut().map(|s| {
        //         s.possible_quorum[try_pre_accept_reply.conflict_instance.unwrap().slot as usize] =
        //             false
        //     });

        //     let mut epaxos_logic = self.epaxos_logic.lock().unwrap();
        //     let mut not_in_quorum = 0;
        //     for q in 0..epaxos_logic.info.n {
        //         if !inst.as_ref().unwrap().from_leader.as_ref().unwrap().possible_quorum
        //             [try_pre_accept_reply.accept_id as usize]
        //         {
        //             not_in_quorum += 1;
        //         }
        //     }

        //     if try_pre_accept_reply.conflict_state.unwrap() >= State::Committed
        //         || not_in_quorum > epaxos_logic.info.n as u32 / 2
        //     {
        //         //abandon recovery, restart from phase 1
        //         //inst.unwrap().from_leader.unwrap().borrow_mut().trying_to_pre_accept = false;
        //         inst.unwrap().from_leader.as_mut().map(|mut s| s.trying_to_pre_accept = false);
        //         // self.start_phase_one(
        //         //     &try_pre_accept_reply.instance,
        //         //     inst.ballot,
        //         //     inst.from_leader.unwrap().client_proposals.unwrap(),
        //         //     ir.unwrap().command,
        //         //     ir.unwrap().command.len() as u32,
        //         // );
        //     }

        //     if not_in_quorum == epaxos_logic.info.n as u32 / 2 {
        //         //this is to prevent defer cycles
        //         let (present, dp, _) = epaxos_logic.deferred_by_instance(
        //             try_pre_accept_reply.instance.replica,
        //             try_pre_accept_reply.instance.slot,
        //         );
        //         if present {
        //             if inst.unwrap().from_leader.as_ref().unwrap().possible_quorum[dp as usize] {
        //                 //an instance whose leader must have been in this instance's quorum has been deferred for this instance => contradiction
        //                 //abandon recovery, restart from phase 1
        //                 // inst.unwrap().from_leader.unwrap().borrow_mut().trying_to_pre_accept = false;
        //                 inst.unwrap().from_leader.as_mut().map(|mut s| s.trying_to_pre_accept = false);
        //                 // self.start_phase_one(
        //                 //     &try_pre_accept_reply.instance,
        //                 //     inst.ballot,
        //                 //     inst.from_leader.unwrap().client_proposals.unwrap(),
        //                 //     ir.unwrap().command,
        //                 //     ir.unwrap().command.len() as u32,
        //                 // );
        //             }
        //         }
        //     }

        //     if inst.as_ref().unwrap().from_leader.as_ref().unwrap().tpa_oks >= epaxos_logic.info.n as u32 / 2 {
        //         //defer recovery and update deferred information
        //         epaxos_logic.update_deferred(
        //             try_pre_accept_reply.instance.replica,
        //             try_pre_accept_reply.instance.slot,
        //             try_pre_accept_reply.conflict_instance.unwrap().replica,
        //             try_pre_accept_reply.conflict_instance.unwrap().slot,
        //         );
        //         inst.unwrap().from_leader
        //             .as_mut()
        //             .map(|mut s| s.trying_to_pre_accept = false);
        //     }
        // }
    }

    pub async fn handle_pre_accept_ok_reply(&self, pacc_ok_reply: PreAcceptOKPayload) {
        info!("Handling PreAcceptok reply");
        let mut epaxos_logic = self.epaxos_logic.lock().unwrap();
        let mut inst = epaxos_logic
            .instance_entry_space
            .get_mut(&pacc_ok_reply.instance)
            .unwrap();

        if inst.state.unwrap() == State::PreAccepted {
            return;
        }

        // XXX: Maybe better??
        let mut epaxos_logic = self.epaxos_logic.lock().unwrap();
        if !epaxos_logic.is_initial_ballot(inst.ballot) {
            return;
        }

        inst.from_leader.as_mut().map(|mut s| s.pre_accept_oks += 1);

        let mut all_commited = true;
        for q in 0..epaxos_logic.info.n {
            let tmp_q = inst.from_leader.as_ref().unwrap().original_deps[q];
            if inst.from_leader.as_ref().unwrap().commited_deps[q] < tmp_q {
                inst.from_leader
                    .as_mut()
                    .map(|s| s.commited_deps[q] = tmp_q);
            }
            if inst.from_leader.as_ref().unwrap().commited_deps[q]
                < epaxos_logic.commited_upto_instance[q]
            {
                inst.from_leader
                    .as_mut()
                    .map(|s| s.commited_deps[q] = epaxos_logic.commited_upto_instance[q]);
            }
            if inst.from_leader.as_ref().unwrap().commited_deps[q] < inst.deps[q] {
                all_commited = false;
            }
        }

        if inst.from_leader.as_ref().unwrap().pre_accept_oks >= epaxos_logic.info.n as u32 / 2
            && inst.from_leader.as_ref().unwrap().all_equal
            && all_commited
            && epaxos_logic.is_initial_ballot(inst.ballot)
        {
            HAPPY.lock().unwrap().add(1);
            info!("fast path in instance{:?}", pacc_ok_reply.instance);
            epaxos_logic
                .instance_entry_space
                .get_mut(&pacc_ok_reply.instance)
                .map(|mut s| s.state = Some(State::Committed));
            epaxos_logic.update_committed(&pacc_ok_reply.instance);
            if !inst
                .from_leader
                .as_ref()
                .unwrap()
                .client_proposals
                .is_none()
                && epaxos_logic.info.dreply
            {
                // TODO: Fix me
                //     // give clients the all clear
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

            epaxos_logic.record_payload_metadata(inst);
            epaxos_logic.sync();

            self.broadcast_commit(
                &pacc_ok_reply.instance,
                &inst.command.as_ref().unwrap(),
                inst.seq,
                inst.deps.clone(),
            )
            .await;
        } else if inst.from_leader.as_ref().unwrap().pre_accept_oks
            >= epaxos_logic.info.n as u32 / 2
        {
            if !all_commited {
                WEIRD.lock().unwrap().add(1);
            }
            SLOW.lock().unwrap().add(1);
            inst.state = Some(State::Accepted);
            self.broadcast_accept(
                &pacc_ok_reply.instance,
                inst.ballot,
                inst.command.as_ref().unwrap().len() as u32,
                inst.seq,
                inst.deps.clone(),
            )
            .await;
        }
        //TODO: take the slow path if messages are slow to arrive
    }

    // fn send_pre_accept_ok(&self,replica_id: u32, pre_accept_ok: PreAcceptOKPayload) {

    // }

    // fn execute(&self) {
    //     info!("Executing");
    //     let epaxos_logic = self.epaxos_logic.lock().unwrap();
    //     //epaxos_logic.execute();
    // }
}

// Handle message function
impl EpaxosService for EpaxosServerImpl {
    // handle preaccept
    fn pre_accept(
        &mut self,
        ctx: ::grpcio::RpcContext,
        req: super::epaxos::PreAcceptPayload,
        sink: ::grpcio::UnarySink<super::epaxos::PreAcceptReplyPayload>,
    ) {
        info!("Received PreAccept");
        let self_inner = Arc::<EpaxosServerInner>::clone(&self.inner);
        let task = async move {
            let mut epaxos_logic = self_inner.epaxos_logic.lock().unwrap();
            let pre_accept_request = PreAccept(PreAcceptPayload::from_grpc(&req));
            let mut inst = if let Some(inst) = epaxos_logic.instance_entry_space.get(&Instance {
                replica: pre_accept_request.0.leader_id,
                slot: pre_accept_request.0.instance.slot,
            }) {
                Some(inst.clone())
            } else {
                None
            };
            // let mut inst = epaxos_logic.instance_entry_space.get(&Instance {
            //     replica: pre_accept_request.0.leader_id,
            //     slot: pre_accept_request.0.instance.slot,
            // });
            let (mut seq, mut deps, mut changed) = epaxos_logic.update_attributes(
                pre_accept_request.0.command.clone(),
                pre_accept_request.0.seq,
                pre_accept_request.0.deps.clone(),
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
                Some(mut instance) => {
                    if pre_accept_request.0.seq > epaxos_logic.max_seq {
                        epaxos_logic.max_seq = pre_accept_request.0.seq + 1;
                    }

                    // 3 -> committed && 4 -> executed
                    if instance.state.unwrap() == State::Committed
                        || instance.state.unwrap() == State::Executed
                    {
                        // reordered handling of commit/accept and pre-accept
                        if instance.command.unwrap().len() == 0 {
                            instance.command = Some(pre_accept_request.0.command.clone());
                            // epaxos_logic.instance_entry_space[&Instance {
                            //     replica: pre_accept_request.0.leader_id,
                            //     slot: pre_accept_request.0.instance.slot,
                            // }]
                            //     .command = Some(pre_accept_request.0.command);
                            epaxos_logic.update_conflicts(
                                &pre_accept_request.0.command,
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
                        instance.command = Some(pre_accept_request.0.command.clone());
                        instance.seq = seq;
                        instance.deps = deps.clone();
                        instance.ballot = pre_accept_request.0.ballot;
                        instance.state = Some(state_change);
                    }
                }
                None => {
                    epaxos_logic.instance_entry_space.insert(
                        pre_accept_request.0.instance,
                        InstanceEntry {
                            ballot: pre_accept_request.0.ballot,
                            command: Some(pre_accept_request.0.command.clone()),
                            seq,
                            deps: deps.clone(),
                            instance: pre_accept_request.0.instance,
                            state: Some(state_change),
                            from_leader: None,
                        },
                    );
                }
            }

            epaxos_logic.update_conflicts(
                &pre_accept_request.0.command,
                &pre_accept_request.0.instance,
                pre_accept_request.0.seq,
            );
            epaxos_logic.record_payload_metadata(
                epaxos_logic
                    .instance_entry_space
                    .get(&pre_accept_request.0.instance)
                    .unwrap(),
            );
            epaxos_logic.record_commands(&pre_accept_request.0.command);
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
                    commited_deps: epaxos_logic.commited_upto_instance.clone(),
                };
                sink.success(pre_accept_reply_payload.to_grpc());
            } else {
                let pok = PreAcceptOKPayload {
                    instance: pre_accept_request.0.instance,
                };
                //self_inner.send_pre_accept_ok(pre_accept_request.0.leader_id, pok);
            }
        };
    }

    // handle accept
    fn accept(
        &mut self,
        ctx: ::grpcio::RpcContext,
        req: super::epaxos::AcceptPayload,
        sink: UnarySink<super::epaxos::AcceptReplyPayload>, // 原本是AcceptOKPayload
    ) {
        info!("handle accept");
        let self_inner = Arc::<EpaxosServerInner>::clone(&self.inner);
        let mut epaxos_logic = self_inner.epaxos_logic.lock().unwrap();
        let accept_request = Accept(AcceptPayload::from_grpc(&req));
        let mut inst = if let Some(inst) = epaxos_logic.instance_entry_space.get(&Instance {
            replica: accept_request.0.leader_id,
            slot: accept_request.0.instance.slot,
        }) {
            Some(inst.clone())
        } else {
            None
        };


        if !inst.is_none()
            && (inst.as_ref().unwrap().state == Some(State::Committed)
                || inst.as_ref().unwrap().state == Some(State::Executed))
        {
            // TODO: deal with better, return is not good
            return;
        }

        if accept_request.0.seq > epaxos_logic.max_seq {
            epaxos_logic.max_seq = accept_request.0.seq + 1;
        }

        if accept_request.0.instance.slot
            > epaxos_logic.current_instance[accept_request.0.instance.replica as usize]
        {
            // update current_instance in the epaxos
            epaxos_logic.current_instance[accept_request.0.instance.replica as usize] =
                accept_request.0.instance.slot + 1;
        }

        match inst {
            Some(mut instance_entry) => {
                if accept_request.0.ballot < instance_entry.ballot {
                    // reply accept
                }
                instance_entry.state = Some(State::Accepted);
                instance_entry.seq = accept_request.0.seq;
                instance_entry.deps = accept_request.0.deps;

                epaxos_logic.instance_entry_space.insert(Instance {
                    replica: accept_request.0.leader_id,
                    slot: accept_request.0.instance.slot,
                }, instance_entry);
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

        epaxos_logic.record_payload_metadata(
            epaxos_logic
                .instance_entry_space
                .get(&accept_request.0.instance)
                .unwrap(),
        );
        epaxos_logic.sync();
        let reply = AcceptReplyPayload {
            instance: accept_request.0.instance,
            ok: 1,
            ballot: accept_request.0.ballot,
        };
        // XXX: Does success method return reply to the original host??
        sink.success(reply.to_grpc());
    }

    // handle commit
    fn commit(
        &mut self,
        ctx: ::grpcio::RpcContext,
        req: super::epaxos::CommitPayload,
        sink: ::grpcio::UnarySink<Empty>,
    ) {
        info!("handle Commit");
        let self_inner = Arc::<EpaxosServerInner>::clone(&self.inner);
        let mut epaxos_logic = self_inner.epaxos_logic.lock().unwrap();
        let commit_payload = CommitPayload::from_grpc(&req);
        let inst = if let Some(inst) = epaxos_logic.instance_entry_space.get(&commit_payload.instance) {
            Some(inst.clone())
        } else {
            None
        };
        // let mut inst = epaxos_logic
        //     .instance_entry_space
        //     .get_mut(&commit_payload.instance);

        if commit_payload.seq > epaxos_logic.max_seq {
            epaxos_logic.max_seq = commit_payload.seq + 1;
        }

        if commit_payload.instance.slot
            > epaxos_logic.current_instance[commit_payload.instance.replica as usize]
        {
            epaxos_logic.current_instance[commit_payload.instance.replica as usize] =
                commit_payload.instance.slot + 1;
        }

        match inst {
            Some(mut instance) => {
                if !instance.from_leader.is_none()
                    && !instance.from_leader.as_ref().unwrap().client_proposals.is_none()
                    && commit_payload.command.len() == 0
                {
                    // someone committed a NO-OP, but we have proposals for this instance
                    // try in a different instance
                    // for _, p := range inst.lb.clientProposals {
                    //     r.ProposeChan <- p
                    // }
                    instance.from_leader = None;
                }
                instance.seq = commit_payload.seq;
                instance.deps = commit_payload.deps;
                instance.state = Some(State::Committed);
                epaxos_logic.instance_entry_space.insert(commit_payload.instance, instance);
            }
            None => {
                epaxos_logic.instance_entry_space.insert(
                    commit_payload.instance,
                    InstanceEntry {
                        ballot: 0,
                        command: Some(commit_payload.command.clone()),
                        seq: commit_payload.seq,
                        deps: commit_payload.deps,
                        instance: commit_payload.instance,
                        state: Some(State::Committed),
                        from_leader: None,
                    },
                );
                epaxos_logic.update_conflicts(
                    &commit_payload.command,
                    &commit_payload.instance,
                    commit_payload.seq,
                );

                if commit_payload.command.len() == 0 {
                    // checkpoint
                    // update latest checkpoint info
                    epaxos_logic.lastest_cp_replica = commit_payload.instance.replica;
                    epaxos_logic.lastest_cp_instance = commit_payload.instance.slot;

                    // discard dependency hashtables
                    epaxos_logic.clear_hashtables();
                }
            }
        }

        epaxos_logic.update_committed(&commit_payload.instance);
        epaxos_logic.record_payload_metadata(
            epaxos_logic
                .instance_entry_space
                .get(&commit_payload.instance)
                .unwrap(),
        );
        epaxos_logic.record_commands(&commit_payload.command);

        let respond = Empty::new();
        sink.success(respond);
    }

    // handle prepare
    fn prepare(
        &mut self,
        ctx: ::grpcio::RpcContext,
        req: super::epaxos::PreparePayload,
        sink: ::grpcio::UnarySink<super::epaxos::PrepareReplyPayload>,
    ) {
        let self_inner = Arc::<EpaxosServerInner>::clone(&self.inner);
        let mut epaxos_logic = self_inner.epaxos_logic.lock().unwrap();
        let prepare_request = Prepare(PreparePayload::from_grpc(&req));
        let inst= if let Some(inst) = epaxos_logic.instance_entry_space.get(&prepare_request.0.instance) {
            Some(inst.clone())
        } else {
            None
        };
        // let mut inst = epaxos_logic
        //     .instance_entry_space
        //     .get(&prepare_request.0.instance);
        let mut nil_deps: Vec<u32> = Vec::new();

        match inst {
            None => {
                epaxos_logic.instance_entry_space.insert(
                    prepare_request.0.instance,
                    InstanceEntry {
                        ballot: prepare_request.0.ballot,
                        command: None,
                        seq: 0,
                        deps: nil_deps.clone(),
                        instance: prepare_request.0.instance,
                        state: None,
                        from_leader: Some(CommandLeaderBookKeeping::default()),
                    },
                );

                let respond = PrepareReplyPayload {
                    accept_id: epaxos_logic.info.id,
                    ok: 1,
                    instance: prepare_request.0.instance,
                    ballot: u32::MAX, // -1
                    state: None,
                    command: Vec::new(),
                    seq: u32::MAX, // -1
                    deps: nil_deps.clone(),
                };
                sink.success(respond.to_grpc());
            }
            Some(mut instance) => {
                let mut ok = 1;
                if prepare_request.0.ballot < instance.ballot {
                    ok = 0;
                } else {
                    instance.ballot = prepare_request.0.ballot;
                }
                let respond = PrepareReplyPayload {
                    accept_id: epaxos_logic.info.id,
                    ok,
                    instance: prepare_request.0.instance,
                    ballot: instance.ballot,
                    state: Some(instance.state.unwrap()),
                    command: instance.command.as_ref().unwrap().to_vec(),
                    seq: instance.seq,
                    deps: instance.deps.clone(),
                };

                epaxos_logic.instance_entry_space.insert(
                    prepare_request.0.instance,
                    instance,
                );
                sink.success(respond.to_grpc());
            }
        }
    }

    // handle commit short
    fn commitshort(
        &mut self,
        ctx: grpcio::RpcContext,
        req: crate::classic::epaxos::CommitShortPayload,
        sink: grpcio::UnarySink<crate::classic::epaxos::Empty>,
    ) {
        info!("handle Commit-short");
        let self_inner = Arc::<EpaxosServerInner>::clone(&self.inner);
        let mut epaxos_logic = self_inner.epaxos_logic.lock().unwrap();
        let commit_short_payload = CommitShortPayload::from_grpc(&req);

        let inst = if let Some(inst) = epaxos_logic.instance_entry_space.get(&commit_short_payload.instance) {
            Some(inst.clone())
        }else {
            None
        };
        // let mut inst = epaxos_logic
        //     .instance_entry_space
        //     .get_mut(&commit_short_payload.instance);

        if commit_short_payload.instance.slot
            > epaxos_logic.current_instance[commit_short_payload.instance.replica as usize]
        {
            epaxos_logic.current_instance[commit_short_payload.instance.replica as usize] =
                commit_short_payload.instance.slot + 1;
        }

        match inst {
            Some(mut instance) => {
                if !instance.from_leader.is_none()
                    && instance.from_leader.as_ref().unwrap().client_proposals.is_none()
                {
                    // try command in a different instance
                    // TODO: fix this
                    // for _, p := range inst.lb.clientProposals {
                    //     r.ProposeChan <- p
                    // }
                    instance.from_leader = None;
                }
                instance.seq = commit_short_payload.seq;
                instance.deps = commit_short_payload.deps;
                instance.state = Some(State::Committed);
                epaxos_logic.instance_entry_space.insert(commit_short_payload.instance, instance);
            }
            None => {
                epaxos_logic.instance_entry_space.insert(
                    commit_short_payload.instance,
                    InstanceEntry {
                        ballot: 0,
                        command: None,
                        seq: commit_short_payload.seq,
                        deps: commit_short_payload.deps,
                        instance: commit_short_payload.instance,
                        state: Some(State::Committed),
                        from_leader: None,
                    },
                );

                if commit_short_payload.count == 0 {
                    // checkpoint
                    // update latest checkpoint info
                    epaxos_logic.lastest_cp_replica = commit_short_payload.instance.replica;
                    epaxos_logic.lastest_cp_instance = commit_short_payload.instance.slot;

                    // discard dependency hashtables
                    epaxos_logic.clear_hashtables();
                }
            }
        }

        epaxos_logic.update_committed(&commit_short_payload.instance);
        epaxos_logic.record_payload_metadata(
            epaxos_logic
                .instance_entry_space
                .get(&commit_short_payload.instance)
                .unwrap(),
        );

        let respond = Empty::new();
        sink.success(respond);
    }

    // handle propose
    fn propose(
        &mut self,
        ctx: grpcio::RpcContext,
        req: crate::classic::epaxos::ProposePayload,
        sink: grpcio::UnarySink<crate::classic::epaxos::Empty>,
    ) {
        //TODO： handle client retry

        //let mut epaxos_logic = self.epaxos_logic.lock().unwrap();
        let self_inner = Arc::<EpaxosServerInner>::clone(&self.inner);
        let mut epaxos_logic = self_inner.epaxos_logic.lock().unwrap();

        let mut queue = QUEUE_BUFFER.lock().unwrap();
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
            .get(epaxos_logic.info.id as usize)
            .unwrap();
        //epaxos_logic.current_instance[epaxos_logic.info.id as usize] += 1;

        info!("Starting instance {:?}", inst_no);
        //info!("Batching is {:?}", batch_size);

        // let mut cmds: Vec<Command> = Vec::with_capacity(batch_size as usize);
        // let mut proposals: Vec<ProposePayload> = Vec::with_capacity(batch_size as usize);
        // for i in 0..batch_size {
        //     let prop = epaxos_logic.propose_chan.pop();
        //     cmds.insert(i as usize, prop.command);
        //     proposals.insert(i as usize, prop);
        // }

        // self_inner.start_phase_one(
        //     &Instance {
        //         replica: epaxos_logic.info.id.0,
        //         slot: *inst_no,
        //     },
        //     0,
        //     ProposePayload::from_grpc(&req),
        //     //batch_size,
        // );

        let respond = Empty::new();
        sink.success(respond);
    }

    // handle try pre accept
    fn try_pre_accept(
        &mut self,
        ctx: grpcio::RpcContext,
        req: crate::classic::epaxos::TryPreAcceptPayload,
        sink: grpcio::UnarySink<crate::classic::epaxos::TryPreAcceptReplyPayload>,
    ) {
        let self_inner = Arc::<EpaxosServerInner>::clone(&self.inner);
        let mut epaxos_logic = self_inner.epaxos_logic.lock().unwrap();
        let mut try_pre_accept_request = TryPreAccept(TryPreAcceptPayload::from_grpc(&req));
        let mut inst = if let Some(inst) = epaxos_logic.instance_entry_space.get(&try_pre_accept_request.0.instance) {
            Some(inst.clone())
        } else {
            None
        };
        let mut try_pre_accept_reply: TryPreAcceptReplyPayload = unsafe{ MaybeUninit::uninit().assume_init() };
        // let mut inst = epaxos_logic
        //     .instance_entry_space
        //     .get(&try_pre_accept_request.0.instance);

        if !inst.is_none() && inst.as_ref().unwrap().ballot > try_pre_accept_request.0.ballot {
            // ballot number too small
            // TODO: lack leader id
            try_pre_accept_reply = TryPreAcceptReplyPayload {
                accept_id: epaxos_logic.info.id,
                instance: try_pre_accept_request.0.instance,
                ok: 0,
                ballot: inst.as_ref().unwrap().ballot,
                conflict_instance: Some(try_pre_accept_request.0.instance),
                conflict_state: Some(inst.as_ref().unwrap().state.unwrap()),
            };
        }

        let (mut conflicts, mut conf_instance, _) = epaxos_logic.find_pre_accept_conflicts(
            &try_pre_accept_request.0.command,
            try_pre_accept_request.0.instance,
            try_pre_accept_request.0.seq,
            &try_pre_accept_request.0.deps,
        );
        if conflicts {
            // TODO: lack leader id
            try_pre_accept_reply = TryPreAcceptReplyPayload {
                accept_id: epaxos_logic.info.id,
                instance: try_pre_accept_request.0.instance,
                ok: 0,
                ballot: inst.unwrap().ballot,
                conflict_instance: Some(Instance {
                    replica: try_pre_accept_request.0.instance.replica,
                    slot: conf_instance,
                }),
                conflict_state: Some(
                    epaxos_logic
                        .instance_entry_space
                        .get(&Instance {
                            replica: try_pre_accept_request.0.instance.replica,
                            slot: conf_instance,
                        })
                        .unwrap()
                        .state
                        .unwrap(),
                ),
            };
            //sink.success(try_pre_accept_reply.to_grpc());
        } else {
            // can pre-accept
            if try_pre_accept_request.0.instance.slot
                >= epaxos_logic.current_instance[try_pre_accept_request.0.instance.replica as usize]
            {
                epaxos_logic.current_instance[try_pre_accept_request.0.instance.replica as usize] =
                    try_pre_accept_request.0.instance.slot + 1;
            }

            match inst {
                Some(ref mut instance_entry) => {
                    instance_entry.command = Some(try_pre_accept_request.0.command);
                    instance_entry.deps = try_pre_accept_request.0.deps.clone();
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

            // TODO: lack leader id
            try_pre_accept_reply = TryPreAcceptReplyPayload {
                accept_id: epaxos_logic.info.id,
                instance: try_pre_accept_request.0.instance,
                ok: 1,
                ballot: inst.unwrap().ballot,
                conflict_instance: None,
                conflict_state: None, // 同上
            };
            //sink.success(try_pre_accept_reply.to_grpc());
        }
        sink.success(try_pre_accept_reply.to_grpc());
    }
}
