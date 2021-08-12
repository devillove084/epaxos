use crate::execute::Executor;

use super::config::REPLICAS_NUM;
use log::info;
use std::{
    cmp,
    cmp::Ordering,
    collections::{BTreeMap, HashMap, HashSet},
    fmt,
};
#[derive(PartialEq, Eq, Hash, Clone, Debug, Copy)]
pub struct ReplicaId(pub u32);

#[derive(Debug, Clone, Default)]
pub struct WriteRequest {
    pub key: String,
    pub value: i32,
}

// TODO: If you want to expose a public interface to different types of requests,
// then it must be based on a "virtual base class" to achieve this effect,
// and a public interface for different external read requests or write requests.
#[derive(Clone, Copy)]
pub struct WriteResponse {
    pub commit: bool,
}

#[derive(Clone)]
pub struct ReadRequest {
    pub key: String,
}

#[derive(Clone, Copy)]
pub struct ReadResponse {
    pub value: i32,
}

#[derive(Clone, PartialEq, Eq, Copy)]
pub enum State {
    PreAccepted,
    Accepted,
    Committed,
    Executed,
}

impl State {
    pub fn change_executed(self) -> Self {
        if self.eq(&Self::Committed) {
            return Self::Executed;
        } else {
            self
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Payload {
    pub ballot: u32,
    pub write_req: WriteRequest,
    pub seq: u32,
    pub deps: Vec<Instance>,
    pub instance: Instance,
}

pub struct PreparePayload {
    pub ballot: u32,
    pub instance: Instance,
}

pub struct PrepareOKPayload {
    pub write_req: WriteRequest,
    pub ballot: u32,
    pub instance: Instance,
}

#[derive(Clone)]
pub struct AcceptOKPayload {
    pub write_req: WriteRequest,
    pub instance: Instance,
}

#[derive(Clone)]
pub struct LogEntry {
    //TODO：It is best not to expose the specific structure of kv here.
    pub key: String,
    pub value: i32,
    pub seq: u32,
    pub deps: Vec<Instance>,
    pub state: State,
}

#[derive(Default, Clone, PartialEq, Copy, Eq, Hash, PartialOrd, Ord)]
pub struct Instance {
    //TODO:In each epoch, there is a different instance version,
    // so before implementing the membership change,
    //the epoch mechanism needs to be implemented first
    pub replica: u32,
    pub slot: u32,
}

#[derive(Debug, Default)]
pub struct Epoch {
    pub epoch: u32,
    pub ballot: u32,
    pub replica: u32,
}

pub struct PreAccept(pub Payload);

pub struct Accept(pub Payload);

pub struct Commit(pub Payload);

pub struct PreAcceptOK(pub Payload);

pub struct AcceptOK(pub AcceptOKPayload);

pub struct Prepare(pub PrepareOKPayload);

pub enum Path {
    Slow(Payload),
    Fast(Payload),
}

pub enum PrepareStage {
    Commit(Payload),
    PaxosAccept(Payload),
    PhaseOne(Payload),
}

pub fn sort_instances(inst1: &Instance, inst2: &Instance) -> Ordering {
    if inst1.replica < inst2.replica {
        Ordering::Less
    } else if inst1.replica > inst2.replica {
        Ordering::Greater
    } else {
        if inst1.slot < inst2.slot {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    }
}

pub struct EpaxosLogic {
    pub id: ReplicaId,
    pub cmds: Vec<HashMap<usize, LogEntry>>,
    pub instance_number: u32,
    pub ballot: u32,
    pub exec: Executor,
    pub epoch_: Epoch,
}

impl EpaxosLogic {
    pub fn init(id: ReplicaId) -> EpaxosLogic {
        let commands = vec![HashMap::new(); REPLICAS_NUM];
        EpaxosLogic {
            id,
            ballot: 0,
            cmds: commands,
            instance_number: 0,
            exec: Executor::default(),
            epoch_: Epoch::default(),
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

     // TODO: put in execute entry

    pub fn _execute(&mut self, instance: &Instance){
        let mut gr_map = Vec::new();
        let mut seq_slot = BTreeMap::new();
        self.exec.build_graph(instance, &mut gr_map, &mut seq_slot);

        // Construct the slot -> Graph
        let mut bs = BTreeMap::new();
        let mut deps = Vec::new();
        for dep in self.cmds[instance.replica as usize].get(&(instance.slot as usize)).unwrap().deps.iter() {
            deps.push(dep.slot as usize);
        }
        
        bs.insert(instance.slot as usize,  deps);
        self.exec.graph = gr_map;
        self.exec.vertices = bs;
        self.exec.seq_slot = seq_slot;
        //self.exec.cmds = self.cmds[instance.replica as usize].clone();

        // execute
        self.exec.execute()
    }

    pub fn lead_consensus(&mut self, write_req: WriteRequest) -> Payload {
        let slot = self.instance_number;
        // TODO: ballot epoch (epoch.0.R) is implicit at beginning of every instance
        let ballot = self.ballot;
        let interf = self.find_interference(&write_req.key);
        let seq = 1 + self.find_max_seq(&interf);
        let log_entry = LogEntry {
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
            // maybe init a unqiue ballot
            ballot: self.ballot,
            write_req,
            seq,
            deps: interf,
            instance: Instance {
                replica: self.id.0,
                slot,
            },
        }
    }

    pub fn decide_path(&self, pre_accept_oks: Vec<Payload>, payload: &Payload) -> Path {
        // TODO: We have to judge the pre_accepts_ok vector len at least n/2 - 1
        let mut new_payload = payload.clone();
        let mut path = Path::Fast(payload.clone());
        for pre_accept_ok in pre_accept_oks {
            let Payload {
                ballot,
                write_req: _,
                seq,
                deps,
                instance: _,
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
        } = payload;
        self.instance_number += 1;
        let log_entry = LogEntry {
            key: write_req.key,
            value: write_req.value,
            seq: seq,
            deps: deps,
            state: State::Committed,
        };
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
        } = payload;
        // we should judge ballot number is larger or smaller

        let log_entry = LogEntry {
            //TODO: fix the ballot judge
            key: write_req.key,
            value: write_req.value,
            seq: seq,
            deps: deps,
            state: State::Accepted,
        };
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
        } = pre_accept_req.0;
        let WriteRequest { key, value } = write_req.clone();
        info!("Processing PreAccept for key: {}, value: {}", key, value);
        let interf = self.find_interference(&key);
        let seq_ = cmp::max(seq, 1 + self.find_max_seq(&interf));
        if interf != deps {
            deps = self.union_deps(deps, interf);
        }
        let log_entry = LogEntry {
            key: key.to_owned(),
            value: value,
            seq: seq_,
            deps: deps.clone(),
            state: State::PreAccepted,
        };
        // update cmd
        self.update_log(log_entry, &instance);
        if self.ballot > ballot {
            return PreAcceptOK(Payload {
                ballot: self.ballot,
                write_req: write_req,
                seq: seq_,
                deps: deps,
                instance: instance,
            });
        } else {

        }
        PreAcceptOK(Payload {
            ballot,
            write_req: write_req,
            seq: seq_,
            deps: deps,
            instance: instance,
        })
    }

    pub fn aware_ballot(&mut self, payload: &Payload) -> Payload {
        let deps = self.cmds[payload.instance.replica as usize].get(&(payload.instance.slot as usize)).unwrap().deps;
        
        //TODO: how do i know the max proposql id in my replica
        // how do i construct the Prepare(epoch.(b+1).Q, instance) in the payload.
        Payload::default()
    }

    pub fn decide_prepare(&mut self, replies: Vec<PrepareOKPayload>, payload: &Payload) -> PrepareStage {
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

    pub fn decide_prepare_ok(&mut self, info: Payload) -> Result<PrepareOKPayload, std::fmt::Error> {
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
        } = accept_req.0;
        let WriteRequest { key, value } = write_req.clone();
        let log_entry = LogEntry {
            key: key,
            value: value,
            seq: seq,
            deps: deps,
            state: State::Accepted,
        };
        self.update_log(log_entry, &instance);
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
        } = commit_req.0;
        let log_entry = LogEntry {
            key: write_req.key,
            value: write_req.value,
            seq: seq,
            deps: deps,
            state: State::Committed,
        };
        // Update the state in the log to commit
        self.update_log(log_entry, &instance);
        info!("Committed. My log is {:#?}", self.cmds);
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
