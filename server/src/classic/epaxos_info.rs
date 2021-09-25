#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_assignments)]

use crate::{classic::{
    execute::Executor,
    message::{
        Command, CommandLeaderBookKeeping, CoreInfo, Instance, InstanceEntry, LogEntry, Operation,
        PreAcceptReplyPayload, PrepareReplyPayload, ReplicaId, State,
    },
    server::EpaxosServerInner,
}, cli::cli::CLIParam};

use super::config::REPLICAS_NUM;
use log::info;
use std::{
    cmp,
    collections::{BTreeMap, HashMap, HashSet},
    fmt,
    ops::{Shl, Shr},
};

pub struct EpaxosLogic {
    //pub id: ReplicaId,
    pub cmds: Vec<HashMap<usize, LogEntry>>,
    //pub instance_number: u32,
    // instance -> ballot
    //pub inst_ballot: BTreeMap<Instance, u32>,
    pub exec: Executor,
    pub info: CoreInfo,
    pub instance_entry_space: BTreeMap<Instance, InstanceEntry>,
    pub current_instance: Vec<u32>,
    pub commited_upto_instance: Vec<u32>,
    pub execed_upto_instance: Vec<u32>,
    pub conflicts: Vec<BTreeMap<String, u32>>,
    pub max_seq_per_key: BTreeMap<String, u32>,
    pub max_seq: u32,
    pub lastest_cp_replica: u32,
    pub lastest_cp_instance: u32,
    pub instance_to_revovery: Vec<Instance>,
}

impl EpaxosLogic {
    // id int, peerAddrList []string, thrifty bool, exec bool, dreply bool, beacon bool, durable bool
    pub fn init(
        param: CLIParam
        // id: ReplicaId,
        // peerAddrList: Vec<String>,
        // thrifty: bool,
        // exec: bool,
        // dreply: bool,
        // beacon: bool,
        // durable: bool,
    ) -> EpaxosLogic {
        let commands = vec![HashMap::new(); REPLICAS_NUM];
        let info = CoreInfo {
            n: REPLICAS_NUM,
            id: param.replica_id,
            //peer_addr_list: param.peer_addr_list,
            peers: Vec::new(),
            peer_readers: Vec::new(),
            peer_writers: Vec::new(),
            alive: BTreeMap::new(),
            state: None,
            shutdown: false,
            thrifty: param.thrifty,
            beacon: param.beacon,
            durable: param.durable,
            stable_store: None,
            preferred_peer_order: Vec::new(),
            ewma: Vec::new(),
            on_client_connect: false,
            dreply: param.dreply,
            exec: param.exec,
            defer_map: BTreeMap::new(),
            tpa_payload: None,
        };
        // spawn run to start the core logic

        EpaxosLogic {
            cmds: commands,
            //instance_number: 0,
            exec: Executor::default(),
            //inst_ballot: BTreeMap::new(),
            info,
            current_instance: Vec::new(),
            commited_upto_instance: Vec::new(),
            execed_upto_instance: Vec::new(),
            conflicts: Vec::new(),
            max_seq_per_key: BTreeMap::new(),
            max_seq: 0,
            lastest_cp_replica: 0,
            lastest_cp_instance: 0,
            instance_to_revovery: Vec::new(),
            instance_entry_space: BTreeMap::new(),
        }
    }

    // pub fn _handle_prepare_reply(&self, preply: &PrepareReplyPayload, xxx: &EpaxosServerInner) {
    //     if let Some(inst) = self.instance_entry_space.get_mut(&preply.instance) {
    //         //let mut epaxos_logic = self.epaxos_logic.lock().unwrap();
    //         let preparing = inst.from_leader.as_ref().unwrap().preparing;
    //         if inst.from_leader.is_none() || !preparing {
    //             // TODO: fix return
    //             // we've moved on -- these are delayed replies, so just ignore
    //             // TODO: should replies for non-current ballots be ignored?
    //             return;
    //         }

    //         if preply.ok == 0 {
    //             // TODO: there is probably another active leader, back off and retry later
    //             //inst.from_leader.as_ref().unwrap().borrow_mut().nacks += 1;
    //             inst.from_leader.as_mut().map(|s| s.nacks += 1);
    //         }

    //         if inst.from_leader.as_ref().unwrap().prepare_oks < self.info.n as u32 / 2 {
    //             // return or maybe record this as log
    //             return;
    //         }

    //         //Got an ACK (preply.OK == TRUE)

    //         inst.from_leader.as_mut().map(|mut s| s.prepare_oks += 1);

    //         if inst.state.unwrap() == State::Committed || inst.state.unwrap() == State::Executed {
    //             self.instance_entry_space.insert(
    //                 preply.instance,
    //                 InstanceEntry {
    //                     ballot: inst.ballot,
    //                     command: Some(preply.command.clone()),
    //                     seq: preply.seq,
    //                     deps: preply.deps.clone(),
    //                     instance: preply.instance,
    //                     state: Some(State::Committed),
    //                     from_leader: None,
    //                 },
    //             );

    //             //broadcastcommit
    //             xxx.broadcast_commit(
    //                 &preply.instance,
    //                 &inst.command.as_ref().unwrap(),
    //                 preply.seq,
    //                 preply.deps.clone(),
    //             );
    //             return;
    //         }

    //         //let ri = inst.from_leader.as_mut().unwrap().recovery_insts.as_mut();
    //         if preply.state == Some(State::Accepted) {
    //             if inst
    //                 .from_leader
    //                 .as_ref()
    //                 .unwrap()
    //                 .recovery_insts
    //                 .as_ref()
    //                 .is_none()
    //                 || inst.from_leader.as_ref().unwrap().max_recv_ballot < preply.ballot
    //             {
    //                 inst.from_leader.as_mut().map(|mut s| {
    //                     s.recovery_insts.as_mut().map(|mut re| {
    //                         re.command = preply.command.clone();
    //                         re.state = preply.state.unwrap();
    //                         re.seq = preply.seq;
    //                         re.deps = preply.deps.clone();
    //                         re.pre_accept_count = 0;
    //                         re.command_leader_response = false;
    //                     });
    //                     s.max_recv_ballot = preply.ballot;
    //                 });
    //             }
    //         }

    //         if (preply.state == Some(State::PreAccepted)
    //             && preply.state == Some(State::PreAcceptedEq))
    //             && (inst
    //                 .from_leader
    //                 .as_ref()
    //                 .unwrap()
    //                 .recovery_insts
    //                 .as_ref()
    //                 .is_none()
    //                 || inst
    //                     .from_leader
    //                     .as_ref()
    //                     .unwrap()
    //                     .recovery_insts
    //                     .as_ref()
    //                     .unwrap()
    //                     .state
    //                     < State::Accepted)
    //         {
    //             if inst
    //                 .from_leader
    //                 .as_mut()
    //                 .unwrap()
    //                 .recovery_insts
    //                 .as_mut()
    //                 .is_none()
    //             {
    //                 inst.from_leader
    //                     .as_mut()
    //                     .unwrap()
    //                     .recovery_insts
    //                     .as_mut()
    //                     .map(|s| {
    //                         s.command = preply.command.clone();
    //                     });
    //             } else if preply.seq == inst.seq && equal(preply.deps.clone(), inst.deps.clone()) {
    //                 inst.from_leader
    //                     .as_mut()
    //                     .unwrap()
    //                     .recovery_insts
    //                     .as_mut()
    //                     .map(|s| {
    //                         s.pre_accept_count += 1;
    //                     });
    //             } else if preply.state == Some(State::PreAccepted) {
    //                 inst.from_leader
    //                     .as_mut()
    //                     .unwrap()
    //                     .recovery_insts
    //                     .as_mut()
    //                     .map(|s| {
    //                         s.command = preply.command.clone();
    //                         s.state = preply.state.unwrap();
    //                         s.seq = preply.seq;
    //                         s.deps = preply.deps.clone();
    //                         s.pre_accept_count += 1;
    //                         s.command_leader_response = false;
    //                     });
    //             } else if preply.accept_id == preply.instance.replica {
    //                 inst.from_leader
    //                     .as_mut()
    //                     .unwrap()
    //                     .recovery_insts
    //                     .as_mut()
    //                     .map(|s| {
    //                         s.command_leader_response = true;
    //                     });
    //             }
    //         }

    //         if inst.from_leader.as_ref().unwrap().recovery_insts.is_none() {
    //             //at least one replica has (pre-)accepted this instance
    //             if inst
    //                 .from_leader
    //                 .as_ref()
    //                 .unwrap()
    //                 .recovery_insts
    //                 .as_ref()
    //                 .unwrap()
    //                 .state
    //                 == State::Accepted
    //                 || (!inst
    //                     .from_leader
    //                     .as_ref()
    //                     .unwrap()
    //                     .recovery_insts
    //                     .as_ref()
    //                     .unwrap()
    //                     .command_leader_response
    //                     && inst
    //                         .from_leader
    //                         .as_ref()
    //                         .unwrap()
    //                         .recovery_insts
    //                         .as_ref()
    //                         .unwrap()
    //                         .pre_accept_count
    //                         >= self.info.n as u32 / 2
    //                     && (self.info.thrifty
    //                         || inst
    //                             .from_leader
    //                             .as_ref()
    //                             .unwrap()
    //                             .recovery_insts
    //                             .as_ref()
    //                             .unwrap()
    //                             .state
    //                             == State::PreAcceptedEq))
    //             {
    //                 // safe to go to accept phase
    //                 inst.command = Some(
    //                     inst.from_leader
    //                         .as_ref()
    //                         .unwrap()
    //                         .recovery_insts
    //                         .as_ref()
    //                         .unwrap()
    //                         .command
    //                         .clone(),
    //                 );
    //                 inst.seq = inst
    //                     .from_leader
    //                     .as_ref()
    //                     .unwrap()
    //                     .recovery_insts
    //                     .as_ref()
    //                     .unwrap()
    //                     .seq;
    //                 inst.deps = inst
    //                     .from_leader
    //                     .as_ref()
    //                     .unwrap()
    //                     .recovery_insts
    //                     .as_ref()
    //                     .unwrap()
    //                     .deps
    //                     .clone();
    //                 inst.from_leader.as_mut().map(|s| {
    //                     s.preparing = false;
    //                 });

    //                 // broadcast accepts
    //                 xxx.broadcast_accept(
    //                     &preply.instance,
    //                     inst.ballot,
    //                     inst.command.as_ref().unwrap().len() as u32,
    //                     inst.seq,
    //                     inst.deps.clone(),
    //                 );
    //             } else if !inst
    //                 .from_leader
    //                 .as_ref()
    //                 .unwrap()
    //                 .recovery_insts
    //                 .as_ref()
    //                 .unwrap()
    //                 .command_leader_response
    //                 && inst
    //                     .from_leader
    //                     .as_ref()
    //                     .unwrap()
    //                     .recovery_insts
    //                     .as_ref()
    //                     .unwrap()
    //                     .pre_accept_count
    //                     >= (self.info.n as u32 / 2 + 1) / 2
    //             {
    //                 //send TryPreAccepts
    //                 //but first try to pre-accept on the local replica
    //                 inst.from_leader.as_mut().map(|s| {
    //                     s.pre_accept_oks = 0;
    //                     s.nacks = 0;
    //                     s.possible_quorum.clear();
    //                 });

    //                 for q in 0..self.info.n {
    //                     inst.from_leader.as_mut().map(|s| {
    //                         s.possible_quorum[q] = true;
    //                     });
    //                 }

    //                 let (conflict, replica_q, instance_i) = self.find_pre_accept_conflicts(
    //                     &inst
    //                         .from_leader
    //                         .as_ref()
    //                         .unwrap()
    //                         .recovery_insts
    //                         .as_ref()
    //                         .unwrap()
    //                         .command
    //                         .clone(),
    //                     preply.instance,
    //                     inst.from_leader
    //                         .as_ref()
    //                         .unwrap()
    //                         .recovery_insts
    //                         .as_ref()
    //                         .unwrap()
    //                         .seq,
    //                     inst.from_leader
    //                         .as_ref()
    //                         .unwrap()
    //                         .recovery_insts
    //                         .as_ref()
    //                         .unwrap()
    //                         .deps
    //                         .clone(),
    //                 );
    //                 if conflict {
    //                     if self
    //                         .instance_entry_space
    //                         .get(&Instance {
    //                             replica: replica_q,
    //                             slot: instance_i,
    //                         })
    //                         .unwrap()
    //                         .state
    //                         .unwrap()
    //                         == State::Committed
    //                     {
    //                         // start Phase1 in the initial leader's instance
    //                         // self.start_phase_one(
    //                         //     &preply.instance,
    //                         //     inst.ballot,
    //                         //     inst.from_leader.map(|s| {s.client_proposals}).unwrap().unwrap(),
    //                         //     ir.command,
    //                         //     ir.command.len() as u32,
    //                         // );
    //                         // return;
    //                     } else {
    //                         inst.from_leader.as_mut().map(|s| s.nacks = 1);
    //                         inst.from_leader
    //                             .as_mut()
    //                             .map(|s| s.possible_quorum[self.info.id.0 as usize] = false);
    //                     }
    //                 } else {
    //                     inst.command = Some(
    //                         inst.from_leader
    //                             .as_ref()
    //                             .unwrap()
    //                             .recovery_insts
    //                             .as_ref()
    //                             .unwrap()
    //                             .command
    //                             .clone(),
    //                     );
    //                     inst.seq = inst
    //                         .from_leader
    //                         .as_ref()
    //                         .unwrap()
    //                         .recovery_insts
    //                         .as_ref()
    //                         .unwrap()
    //                         .seq;
    //                     inst.deps = inst
    //                         .from_leader
    //                         .as_ref()
    //                         .unwrap()
    //                         .recovery_insts
    //                         .as_ref()
    //                         .unwrap()
    //                         .deps
    //                         .clone();
    //                     inst.state = Some(State::PreAccepted);
    //                     inst.from_leader.as_mut().map(|s| {
    //                         s.pre_accept_oks = 1;
    //                     });
    //                 }

    //                 inst.from_leader.as_mut().map(|s| s.preparing = false);
    //                 inst.from_leader
    //                     .as_mut()
    //                     .map(|s| s.trying_to_pre_accept = true);
    //                 xxx.broadcast_try_pre_accept(
    //                     &preply.instance,
    //                     inst.ballot,
    //                     inst.command.as_ref().unwrap().to_vec(),
    //                     inst.seq,
    //                     inst.deps.clone(),
    //                 );
    //             } else {
    //                 inst.from_leader.as_mut().map(|s| s.preparing = false);
    //                 // self.start_phase_one(
    //                 //     &preply.instance,
    //                 //     inst.ballot,
    //                 //     inst.from_leader.map(|s| {s.client_proposals}).unwrap().unwrap(),
    //                 //     ir.command,
    //                 //     ir.command.len() as u32,
    //                 // );
    //             }
    //         } else {
    //             let mut noop_deps: Vec<u32> = Vec::new();
    //             noop_deps[preply.instance.replica as usize] = preply.instance.slot - 1;
    //             //inst.from_leader.unwrap().borrow_mut().preparing = false;
    //             inst.from_leader.as_mut().map(|s| s.preparing = false);
    //             self.instance_entry_space.insert(
    //                 preply.instance,
    //                 InstanceEntry {
    //                     ballot: inst.ballot,
    //                     command: None,
    //                     seq: 0,
    //                     deps: noop_deps.clone(),
    //                     instance: preply.instance,
    //                     state: Some(State::Accepted),
    //                     from_leader: inst.from_leader.clone(),
    //                 },
    //             );
    //             xxx.broadcast_accept(&preply.instance, inst.ballot, 0, 0, noop_deps.clone());
    //         }
    //     }
    // }

    pub fn _execute(&mut self, instance: &Instance) {
        let mut gr_map = Vec::new();
        let mut seq_slot = BTreeMap::new();
        self.exec
            .build_graph(instance.slot, &mut gr_map, &mut seq_slot);

        // Construct the slot -> Graph
        let mut bs = BTreeMap::new();
        let mut deps = Vec::new();
        for dep in self.cmds[instance.replica as usize]
            .get(&(instance.slot as usize))
            .unwrap()
            .deps
            .iter()
        {
            deps.push(*dep as usize);
        }

        bs.insert(instance.slot as usize, deps);
        self.exec.graph = gr_map;
        self.exec.vertices = bs;
        self.exec.seq_slot = seq_slot;
        //self.exec.cmds = self.cmds[instance.replica as usize].clone();

        // execute
        self.exec.execute()
    }

    pub fn find_pre_accept_conflicts(
        &mut self,
        cmds: &Vec<Command>,
        instance: Instance,
        seq: u32,
        deps: &Vec<u32>,
    ) -> (bool, u32, u32) {
        if let Some(inst) = self.instance_entry_space.get(&instance) {
            if inst.command.as_ref().unwrap().len() > 0 {
                if inst.state.unwrap() >= State::Accepted {
                    // already ACCEPTED or COMMITTED
                    // we consider this a conflict because we shouldn't regress to PRE-ACCEPTED
                    return (true, instance.replica, instance.slot)
                }
                if inst.seq == self.info.tpa_payload.as_ref().unwrap().seq && equal(inst.deps.clone(), self.info.tpa_payload.as_ref().unwrap().deps.to_vec()) {
                    // already PRE-ACCEPTED, no point looking for conflicts again
                    return (false, instance.replica, instance.slot);
                }
            }
        }
        // if !inst.is_none() && inst.unwrap().command.unwrap().len() > 0 {
        //     if inst.unwrap().state.unwrap() >= State::Accepted {
        //         // already ACCEPTED or COMMITTED
        //         // we consider this a conflict because we shouldn't regress to PRE-ACCEPTED
        //         return (true, instance.replica, instance.slot);
        //     }
        //     if inst.unwrap().seq == self.info.tpa_payload.unwrap().seq
        //         && equal(
        //             inst.unwrap().deps,
        //             self.info.tpa_payload.unwrap().deps.to_vec(),
        //         )
        //     {
        //         // already PRE-ACCEPTED, no point looking for conflicts again
        //         return (false, instance.replica, instance.slot);
        //     }
        // }

        for q in 0..self.info.n {
            for i in self.execed_upto_instance[q]..self.current_instance[q] {
                if instance.replica == q as u32 && instance.slot == i {
                    // no point checking past instance in replica's row, since replica would have
                    // set the dependencies correctly for anything started after instance
                    break;
                }

                if i == deps[q] {
                    // the instance cannot be a dependency for itself
                    continue;
                }

                if let Some(inst) = self.instance_entry_space.get(&Instance {
                        replica: q as u32,
                        slot: i,
                }) {
                    if inst.command.is_none() || inst.command.as_ref().unwrap().len() == 0 {
                        continue;
                    }
                    if inst.deps[instance.replica as usize] > instance.slot {
                        continue;
                    }

                    if self.conflict_batch(inst.command.as_ref().unwrap(), cmds) {
                        if i > deps[q]
                            || (i < deps[q]
                                && inst.seq >= seq
                                && (q != instance.replica as usize
                                    || inst.state.unwrap() > State::PreAcceptedEq))
                        {
                            // this is a conflict
                            return (true, q as u32, i);
                        }
                    }
                } else {
                    continue;
                }
                // let mut inst_s = self.instance_entry_space.get_mut(&Instance {
                //     replica: q as u32,
                //     slot: i,
                // });
                // if inst_s.is_none()
                //     || inst_s.unwrap().command.is_none()
                //     || inst.unwrap().command.unwrap().len() == 0
                // {
                //     continue;
                // }
                // if inst.unwrap().deps[instance.replica as usize] > instance.slot {
                //     // instance q.i depends on instance replica.instance, it is not a conflict
                //     continue;
                // }

                
            }
        }
        return (false, u32::MAX, u32::MAX);
    }

    fn conflict(&self, cmd1: &Command, cmd2: &Command) -> bool {
        if cmd1.key == cmd2.key {
            if cmd1.op == Operation::Put || cmd2.op == Operation::Put {
                return true;
            }
        }
        return false;
    }

    fn conflict_batch(&self, batch1: &Vec<Command>, batch2: &Vec<Command>) -> bool {
        for i in 0..batch1.len() {
            for j in 0..batch2.len() {
                if self.conflict(&batch1[i], &batch2[j]) {
                    return true;
                }
            }
        }
        return false;
    }

    pub fn clear_hashtables(&mut self) {
        for q in 0..self.info.n {
            // XXX: does it satisfied??
            self.conflicts[q].clear();
        }
    }

    pub fn update_attributes(
        &mut self,
        cmds: Vec<Command>,
        mut seq: u32,
        mut deps: Vec<u32>,
        instance: &Instance,
    ) -> (u32, Vec<u32>, bool) {
        let mut changed = false;
        for q in 0..self.info.n {
            if self.info.id != instance.replica && q == instance.replica as usize {
                continue;
            }

            for i in 0..cmds.len() {
                let present = self.conflicts[q].get(&cmds[i].key);
                match present {
                    Some(d) => {
                        if d > &deps[q] {
                            deps[q] = *d;
                            if seq
                                <= self
                                    .instance_entry_space
                                    .get(&Instance {
                                        replica: q as u32,
                                        slot: i as u32,
                                    })
                                    .unwrap()
                                    .seq
                            {
                                seq = self
                                    .instance_entry_space
                                    .get(&Instance {
                                        replica: q as u32,
                                        slot: i as u32,
                                    })
                                    .unwrap()
                                    .seq;
                            }
                            changed = true;
                            break;
                        }
                    }
                    None => {
                        unreachable!();
                    }
                }
            }
        }

        for i in 0..cmds.len() {
            let present = self.max_seq_per_key.get(&cmds[i].key);
            match present {
                Some(s) => {
                    if seq <= *s {
                        changed = true;
                        seq = s + 1;
                    }
                }
                None => {
                    unreachable!();
                }
            }
        }

        return (seq, deps, changed);
    }

    pub fn update_conflicts(&mut self, cmds: &Vec<Command>, instance: &Instance, seq: u32) {
        for i in 0..cmds.len() {
            let present = self.conflicts[instance.replica as usize].get(&cmds[i].key);
            match present {
                Some(d) => {
                    if d < &instance.slot {
                        self.conflicts[instance.replica as usize]
                            .insert(cmds[i].key.clone(), instance.slot);
                    }
                }
                None => {
                    self.conflicts[instance.replica as usize].insert(cmds[i].key.clone(), instance.slot);
                }
            }

            let present = self.max_seq_per_key.get(&cmds[i].key);
            match present {
                Some(s) => {
                    if s < &seq {
                        self.max_seq_per_key.insert(cmds[i].key.clone(), seq);
                    }
                }
                None => {
                    self.max_seq_per_key.insert(cmds[i].key.clone(), seq);
                }
            }
        }
    }

    pub fn record_payload_metadata(&self, instance: &InstanceEntry) {}

    pub fn record_commands(&self, cmds: &Vec<Command>) {}

    pub fn sync(&self) {}

    pub fn update_committed(&mut self, instance: &Instance) {
        let iter_inst = Instance {
            replica: instance.replica,
            slot: self.commited_upto_instance[instance.replica as usize] + 1,
        };
        let iter_inst_entry = self.instance_entry_space.get(&iter_inst);
        while !iter_inst_entry.is_none()
            && (iter_inst_entry.unwrap().state.unwrap() == State::Committed
                || iter_inst_entry.unwrap().state.unwrap() == State::Executed)
        {
            self.commited_upto_instance[instance.replica as usize] += 1;
        }
    }

    pub fn merge_attributes(
        &self,
        mut seq1: u32,
        mut deps1: Vec<u32>,
        seq2: u32,
        deps2: Vec<u32>,
    ) -> (u32, Vec<u32>, bool) {
        let mut equal = true;
        if seq1 != seq2 {
            equal = false;
            if seq2 > seq1 {
                seq1 = seq2;
            }
        }

        for q in 0..self.info.n {
            if q == self.info.id as usize {
                continue;
            }

            if deps1[q] != deps2[q] {
                equal = false;
                if deps2[q] > deps1[q] {
                    deps1[q] = deps2[q];
                }
            }
        }

        return (seq1, deps1, equal);
    }

    pub fn is_initial_ballot(&self, ballot: u32) -> bool {
        ballot >> 4 == 0
    }

    pub fn update_deferred(&mut self, dr: u32, di: u32, r: u32, i: u32) {
        let daux = (dr as u64).shl(32) | (di as u64);
        let aux = (r as u64).shl(32) | (i as u64);
        self.info.defer_map.insert(aux, daux);
    }

    pub fn deferred_by_instance(&self, q: u32, i: u32) -> (bool, u32, u32) {
        let mut aux: u64 = (q as u64).shl(32) | (i as u64);
        let daux = self.info.defer_map.get(&aux);
        let mut dq: u32 = 0;
        let mut di: u32 = 0;
        match daux {
            Some(dd) => {
                dq = (*dd as u32).shr(32);
                di = *dd as u32;
            }
            None => {
                return (false, 0, 0);
            }
        }
        return (true, dq, di);
    }

    // Ballot helper function
    pub fn make_unique_ballot(&self, ballot: u32) -> u32 {
        return (ballot << 4) | self.info.id;
    }

    pub fn make_ballot_larger_than(&self, ballot: u32) -> u32 {
        return self.make_unique_ballot((ballot >> 4) + 1);
    }
}

pub fn equal(a: Vec<u32>, b: Vec<u32>) -> bool {
    if a.len() != b.len() {
        return false;
    }
    for i in 0..a.len() {
        if a[i] != b[i] {
            return false;
        }
    }
    return true;
}

impl fmt::Debug for LogEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            "\nWrite(key = {}, value = {})\nSeq = {}\nDeps = {:#?}\nState = {:?}\n",
            self.key, self.value, self.seq, self.deps, self.state
        )
    }
}

impl fmt::Debug for Instance {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "(Replica: {}, Slot: {})", self.replica, self.slot)
    }
}

impl fmt::Debug for State {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            State::PreAccepted => write!(f, "PreAccepted"),
            State::Accepted => write!(f, "Accepted"),
            State::Committed => write!(f, "Committed"),
            State::Executed => write!(f, "Executed"),
            State::PreAcceptedEq => write!(f, "PreAcceptedEQ"),
        }
    }
}
