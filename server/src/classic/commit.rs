use crate::{execute::Executor, message::{Command, CommandLeaderBookKeeping, CoreInfo, Instance, InstanceEntry, LogEntry, Path, PayloadState, PreAcceptReplyPayload, PrepareReplyPayload, ReplicaId, State}};

use super::config::REPLICAS_NUM;
use log::info;
use std::{cmp, collections::{BTreeMap, HashMap, HashSet}, fmt};

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
    pub fn init(id: ReplicaId, peerAddrList: Vec<String>, thrifty: bool, exec: bool, dreply: bool, beacon: bool, durable: bool) -> EpaxosLogic {
        let commands = vec![HashMap::new(); REPLICAS_NUM];
        let info = CoreInfo {
            n: REPLICAS_NUM,
            id,
            peer_addr_list: peerAddrList,
            peers: Vec::new(),
            peer_readers: Vec::new(),
            peer_writers: Vec::new(),
            alive: BTreeMap::new(),
            state: None,
            shutdown: false,
            thrifty,
            beacon,
            durable,
            stable_store: None,
            preferred_peer_order: Vec::new(),
            ewma: Vec::new(),
            on_client_connect: false,
            dreply,
            exec,
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

    pub fn update_log(&mut self, log_entry: LogEntry, instance: &Instance) {
        info!("updating log..");
        //TODO: Flush executed logs to disks
        let state = self.cmds[instance.replica as usize]
            .get(&(instance.slot as usize))
            .unwrap()
            .state;
        match state {
            State::Committed => {
                // TODO:async????
                self._execute(instance);
                log_entry.state.change_executed();
                self.cmds[instance.replica as usize].insert(instance.slot as usize, log_entry);
            }
            _ => {
                self.cmds[instance.replica as usize].insert(instance.slot as usize, log_entry);
            }
        }
    }

    pub fn _execute(&mut self, instance: &Instance) {
        let mut gr_map = Vec::new();
        let mut seq_slot = BTreeMap::new();
        self.exec.build_graph(instance, &mut gr_map, &mut seq_slot);

        // Construct the slot -> Graph
        let mut bs = BTreeMap::new();
        let mut deps = Vec::new();
        for dep in self.cmds[instance.replica as usize]
            .get(&(instance.slot as usize))
            .unwrap()
            .deps
            .iter()
        {
            deps.push(dep.slot as usize);
        }

        bs.insert(instance.slot as usize, deps);
        self.exec.graph = gr_map;
        self.exec.vertices = bs;
        self.exec.seq_slot = seq_slot;
        //self.exec.cmds = self.cmds[instance.replica as usize].clone();

        // execute
        self.exec.execute()
    }

    pub fn handle_pre_accpet_reply(&mut self, preply_payload: PreAcceptReplyPayload) {
        info!("Handling PreAccept reply");
        let mut inst = self.instance_entry_space.get(&preply_payload.instance).unwrap();

        if inst.state.state_num != 0 {
            // we've moved on, this is a delayed reply
            return;
        }

        if inst.ballot != preply_payload.ballot {
            return;
        }

        if preply_payload.ok == 0  {
            // TODO: there is probably another active leader
            inst.from_leader.nacks += 1;
            if preply_payload.ballot > inst.from_leader.max_recv_ballot {
                inst.from_leader.max_recv_ballot = preply_payload.ballot;
            }
            if inst.from_leader.nacks >= self.info.n as u32 / 2 {
                // TODO:
            }

            return;
        }

        inst.from_leader.prepare_oks += 1;

        let mut equal = false;
        (inst.seq, inst.deps, equal) = self.merge_attributes(inst.seq, inst.deps, preply_payload.seq, preply_payload.deps);
        if (self.info.n <= 3 && !self.info.thrifty) || inst.from_leader.pre_accept_oks > 1 {
            inst.from_leader.all_equal = inst.from_leader.all_equal && equal;
            if !equal {
                conflicts += 1;
            }
        }

        let mut all_commited = true;
        for q in 0..self.info.n {
            if inst.from_leader.commited_deps[q] < preply_payload.commited_deps[q] {
                inst.from_leader.commited_deps[q] = preply_payload.commited_deps[q];
            }
            if inst.from_leader.commited_deps[q] < self.commited_upto_instance[q] {
                inst.from_leader.commited_deps[q] = self.commited_upto_instance[q];
            }
            if inst.from_leader.commited_deps[q] < inst.deps[q] {
                all_commited = false;
            }
        }

        //can we commit on the fast path?
        if inst.from_leader.pre_accept_oks >= self.info.n as u32 / 2 && inst.from_leader.all_equal && all_commited && is_initial_ballot(inst.ballot) {
            happy += 1;
            info!("Fast path for instance");
            // change to committed
            self.instance_entry_space[&preply_payload.instance].state = PayloadState {state_num: 3};
            self.update_committed(preply_payload.instance);

            if inst.from_leader.client_proposals.len() > 0 && !self.info.dreply {
                // give clients the all clear
                for i in 0..inst.from_leader.client_proposals.len() {
                    // TODO：这里处理的不太对
                    self.reply_propose();
                }
            }

        }



    }

    

    pub fn lead_consensus(&mut self, write_req: WriteRequest) -> Payload {
        // lead_consensus is meaning phase one
        let slot = self.instance_number;
        let interf = self.find_interference(&write_req.key);
        let seq = 1 + self.find_max_seq(&interf);
        // ballot number start with 0
        let log_entry = LogEntry {
            ballot: 0,
            key: write_req.key.to_owned(),
            value: write_req.value,
            seq: seq,
            deps: interf.clone(),
            state: State::PreAccepted,
        };
        // Not only update log, actually update cmd space, it is preAccepted
        self.update_log(
            log_entry,
            &Instance {
                replica: self.id.0,
                slot: slot,
            },
        );
        Payload {
            // every write request start with natural number 0
            ballot: 0,
            write_req,
            seq,
            deps: interf,
            instance: Instance {
                replica: self.id.0,
                slot,
            },
            from_leader: CommandLeaderBookKeeping::default(),
        }
    }

    pub fn decide_path(&self, pre_accept_oks: Vec<Payload>, payload: &Payload) -> Path {
        // TODO: We have to judge the pre_accepts_ok vector len at least n/2 - 1
        let mut new_payload = payload.clone();
        let mut path = Path::Fast(payload.clone());
        for pre_accept_ok in pre_accept_oks {
            let Payload {
                ballot,
                seq,
                deps,
                instance: _,
                command,
                state,
                from_leader,
            } = pre_accept_ok.clone();
            if seq == payload.seq && deps == payload.deps {
                continue;
            } else {
                info!("Got some dissenting voice: {:#?}", pre_accept_ok.deps);
                // Union deps from all replies
                let new_deps = self.union_deps(new_payload.deps, pre_accept_ok.deps);
                new_payload.deps = new_deps.clone();
                // Set seq to max of seq from all replies
                if pre_accept_ok.seq > seq {
                    new_payload.seq = pre_accept_ok.seq;
                }
                path = Path::Slow(new_payload.clone());
            }
        }
        path
    }

    pub fn committed(&mut self, payload: Payload) {
        let Payload {
            ballot,
            write_req,
            seq,
            deps,
            instance,
            from_leader,
        } = payload;
        self.instance_number += 1;
        let log_entry = LogEntry {
            ballot,
            key: write_req.key,
            value: write_req.value,
            seq: seq,
            deps: deps,
            state: State::Committed,
        };
        self.inst_ballot.insert(instance, ballot);
        self.update_log(
            log_entry,
            &Instance {
                replica: instance.replica,
                slot: instance.slot,
            },
        );
        info!("Commited. My log is {:#?}", self.cmds);
    }

    pub fn accepted(&mut self, payload: Payload) {
        let Payload {
            ballot,
            write_req,
            seq,
            deps,
            instance,
            from_leader,
        } = payload;
        let log_entry = LogEntry {
            ballot,
            key: write_req.key,
            value: write_req.value,
            seq: seq,
            deps: deps,
            state: State::Accepted,
        };
        self.inst_ballot.insert(instance, ballot);
        self.update_log(
            log_entry,
            &Instance {
                replica: instance.replica,
                slot: instance.slot,
            },
        );
    }

    pub fn union_deps(&self, mut deps1: Vec<Instance>, mut deps2: Vec<Instance>) -> Vec<Instance> {
        deps1.append(&mut deps2);
        deps1.sort_by(sort_instances);
        deps1.dedup();
        deps1
    }

    pub fn pre_accept_(&mut self, pre_accept_req: PreAccept) -> PreAcceptOK {
        let Payload {
            ballot,
            write_req,
            seq,
            mut deps,
            instance,
            from_leader,
        } = pre_accept_req.0;
        let WriteRequest {
            key,
            value,
            request_id,
            timestamp,
        } = write_req.clone();
        info!("Processing PreAccept for key: {}, value: {}", key, value);
        let interf = self.find_interference(&key);
        let seq_ = cmp::max(seq, 1 + self.find_max_seq(&interf));
        if interf != deps {
            deps = self.union_deps(deps, interf);
        }
        let log_entry = LogEntry {
            ballot,
            key: key.to_owned(),
            value: value,
            seq: seq_,
            deps: deps.clone(),
            state: State::PreAccepted,
        };
        // update cmd
        self.update_log(log_entry, &instance);
        self.inst_ballot.insert(instance, ballot);
        PreAcceptOK(Payload {
            ballot,
            write_req: write_req,
            seq: seq_,
            deps: deps,
            instance: instance,
            from_leader,
        })
    }

    pub fn aware_ballot(&mut self, payload: &Payload) -> Payload {
        let deps = self.cmds[payload.instance.replica as usize]
            .get(&(payload.instance.slot as usize))
            .unwrap()
            .deps;

        //TODO: how do i know the max proposql id in my replica
        // how do i construct the Prepare(epoch.(b+1).Q, instance) in the payload.
        
        Payload::default()
    }

    pub fn decide_prepare(
        &mut self,
        replies: Vec<PrepareOKPayload>,
        payload: &Payload,
    ) -> PrepareStage {
        if replies.contain(payload) == State::Committed {
            return PrepareStage::Commit(*payload);
        } else if replies.contain(payload) == State::Accepted {
            return PrepareStage::PaxosAccept(*payload);
        } else if replies.at_least_contain(*payload) == State::PreAccepted {
            return PrepareStage::PaxosAccept(*payload);
        } else if replies.at_least_one_contain(payload) {
            // start Phase 1 (at line 2) for γ at L.i, avoid fast path
            // XXX: This is wrong code.
            return PrepareStage::PhaseOne(*payload);
        }else {
            //TODO: start Phase 1 (at line 2) for no-op at L.i, avoid fast path
            // XXX: This is wrong code
            return PrepareStage::PhaseOne(*payload);
        }
    }

    pub fn reply_prepare_ok(
        &mut self,
        info: Payload,
    ) -> Result<PrepareOKPayload, std::fmt::Error> {
        //TODO: How do i know "epoch.b.Qislargerthanthemostrecentballot number epoch.x.Y accepted for instance L.i "
        return Err(todo!());
    }

    pub fn accept_(&mut self, accept_req: Accept) -> AcceptOK {
        info!("=======ACCEPT========");
        let Payload {
            ballot,
            write_req,
            seq,
            deps,
            instance,
            from_leader,
        } = accept_req.0;
        let WriteRequest {
            key,
            value,
            request_id,
            timestamp,
        } = write_req.clone();
        let log_entry = LogEntry {
            ballot,
            key: key,
            value: value,
            seq: seq,
            deps: deps,
            state: State::Accepted,
        };
        self.update_log(log_entry, &instance);
        self.inst_ballot.insert(instance, ballot);
        AcceptOK(AcceptOKPayload {
            write_req: write_req,
            instance: instance,
        })
    }
    pub fn commit_(&mut self, commit_req: Commit) -> () {
        let Payload {
            ballot,
            write_req,
            seq,
            deps,
            instance,
            from_leader,
        } = commit_req.0;
        let log_entry = LogEntry {
            ballot,
            key: write_req.key,
            value: write_req.value,
            seq: seq,
            deps: deps,
            state: State::Committed,
        };
        // Update the state in the log to commit
        self.inst_ballot.insert(instance, ballot);
        self.update_log(log_entry, &instance);
        info!("Committed. My log is {:#?}", self.cmds);
    }

    fn update_attributes(&self, cmds: Vec<Command>, seq: u32, deps: Vec<Instance>, instance: &Instance) -> (u32, Vec<Instance>, bool) {

    }

    fn find_interference(&self, key: &String) -> Vec<Instance> {
        let mut interf = Vec::new();
        for replica in 0..REPLICAS_NUM {
            for (slot, log_entry) in self.cmds[replica].iter() {
                if log_entry.key == *key {
                    let instance = Instance {
                        replica: replica as u32,
                        slot: *slot as u32,
                    };
                    interf.push(instance);
                }
            }
        }
        interf.sort_by(sort_instances);
        interf
    }

    fn find_max_seq(&self, interf: &Vec<Instance>) -> u32 {
        let mut seq = 0;
        for instance in interf {
            let interf_seq = self.cmds[instance.replica as usize]
                .get(&(instance.slot as usize))
                .unwrap()
                .seq;
            if interf_seq > seq {
                seq = interf_seq;
            }
        }
        seq
    }

    pub fn find_all_instance(&self, instance: &Instance, deps: Vec<Instance>) -> usize {
        let mut hs = HashSet::new();
        hs.insert(instance.slot);
        for i in deps {
            hs.insert(i.slot);
        }
        return hs.len();
    }

    pub fn _recovery_instance(&self, payload: &mut Payload) {

        //TODO: 这里应加上判断是否有状态
        let log_entry = self.cmds[payload.instance.replica as usize].get(&(payload.instance.slot as usize));
        match log_entry {
            Some(lentry) => {
                match lentry.state {
                    State::PreAccepted => {
                        payload.from_leader.recovery_insts = RecoveryPayload {
                            ballot: payload.ballot,
                            write_req: payload.write_req,
                            seq: payload.seq,
                            deps: payload.deps,
                            instance: payload.instance,
                            pre_accept_count: 1,
                            command_leader_response: (self.id.0 == payload.instance.replica),
                        }
                    },
                    State::Accepted => {
                        payload.from_leader.recovery_insts = RecoveryPayload {
                            ballot: payload.ballot,
                            write_req: payload.write_req,
                            seq: payload.seq,
                            deps: payload.deps,
                            instance: payload.instance,
                            pre_accept_count: 0,
                            command_leader_response: false,
                        };
                        payload.from_leader.max_recv_ballot = payload.ballot;
                    },
                    _ => unreachable!(),
                }
            },
            None => todo!(),
        }

        payload.ballot = self.make_ballot_larger_than(payload.ballot);
    }

    // Ballot helper function

    pub fn make_unique_ballot(&self, ballot: u32) -> u32 {
        return (ballot << 4) | self.id.0;
    }

    pub fn make_ballot_larger_than(&self, ballot: u32) -> u32 {
        return self.make_unique_ballot((ballot >> 4) + 1);
    }
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
        }
    }
}
