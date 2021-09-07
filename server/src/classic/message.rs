use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fs::File;


#[derive(PartialEq, Eq, Hash, Clone, Debug, Copy, PartialOrd, Ord)]
pub struct ReplicaId(pub u32);

#[derive(Default, Clone)]
pub struct PayloadState {
    pub state_num: i32,
}

#[derive(Default, Clone)]
pub struct Operation {
    pub op: i32,
}

#[derive(Default, Clone)]
pub struct Command {
    pub op: Operation,
    pub key: String,
    pub value: i32,
}

#[derive(Clone)]
pub struct ProposePayload {
    pub command_id: u32,
    pub command: Command,
    pub timestamp: u64,
}

pub struct ProposeReplyPayload {
    pub ok: bool,
    pub command_id: u32,
    pub value: i32,
    pub timestamp: u64,
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

#[derive(Clone, Default)]
pub struct InstanceEntry {
    pub ballot: u32,
    //pub write_req: WriteRequest,
    pub command: Vec<Command>,
    pub seq: u32,
    pub deps: Vec<Instance>,
    pub instance: Instance,
    pub state: PayloadState,
    pub from_leader: Option<CommandLeaderBookKeeping>,
}

pub struct PreparePayload {
    pub leader_id: u32,
    pub ballot: u32,
    pub instance: Instance,
}

#[derive(Default)]
pub struct PrepareReplyPayload {
    pub accept_id: u32,
    pub ok: u32,
    pub instance: Instance,
    pub ballot: u32,
    pub state: PayloadState,
    pub command: Vec<Command>,
    pub seq: u32,
    pub deps: Vec<Instance>,
}

pub struct PreAcceptPayload {
    pub leader_id: u32,
    pub instance: Instance,
    pub ballot: u32,
    pub command: Vec<Command>,
    pub seq: u32,
    pub deps: Vec<Instance>,
}

pub struct PreAcceptReplyPayload {
    pub instance: Instance,
    pub ok: u32,
    pub ballot: u32,
    pub command: Vec<Command>,
    pub seq: u32,
    pub deps: Vec<Instance>,
    pub commited_deps: Vec<Instance>,
}

pub struct PreAcceptOKPayload {
    pub instance: Instance,
}

pub struct AcceptPayload {
    pub leader_id: u32,
    pub instance: Instance,
    pub ballot: u32,
    pub count: u32,
    pub seq: u32,
    pub deps: Vec<Instance>,
}

pub struct AcceptReplyPayload {
    pub instance: Instance,
    pub ok: u32,
    pub ballot: u32,
}

pub struct CommitPayload {
    pub leader_id: u32,
    pub instance: Instance,
    pub command: Vec<Command>,
    pub seq: u32,
    pub deps: Vec<Instance>,
}

pub struct CommitShortPayload {
    pub leader_id: u32,
    pub instance: Instance,
    pub count: u32,
    pub seq: u32,
    pub deps: Vec<Instance>,
}

pub struct TryPreAcceptPayload {
    pub leader_id: u32,
    pub instance: Instance,
    pub ballot: u32,
    pub command: Vec<Command>,
    pub seq: u32,
    pub deps: Vec<Instance>,
}

pub struct TryPreAcceptReplyPayload {
    pub accept_id: u32,
    pub instance: Instance,
    pub ok: u32,
    pub ballot: u32,
    pub conflict_instance: Option<Instance>,
    pub conflict_state: Option<State>,
}



// #[derive(Clone)]
// pub struct AcceptOKPayload {
//     //pub write_req: WriteRequest,
//     pub command: Command,
//     pub instance: Instance,
// }

#[derive(Clone)]
pub struct LogEntry {
    //TODO：It is best not to expose the specific structure of kv here.
    pub key: String,
    pub value: i32,
    pub seq: u32,
    pub deps: Vec<Instance>,
    pub state: State,
    pub ballot: u32,
}

#[derive(Default, Clone, PartialEq, Copy, Eq, Hash, PartialOrd, Ord)]
pub struct Instance {
    //TODO:In each epoch, there is a different instance version,
    // so before implementing the membership change,
    //the epoch mechanism needs to be implemented first
    pub replica: u32,
    pub slot: u32,
}

// #[derive(Debug, Default)]
// pub struct Epoch {
//     pub epoch: u32,
//     pub ballot: u32,
//     pub replica: u32,
// }

pub struct PreAccept(pub PreAcceptPayload);

pub struct Prepare(pub PreparePayload);

pub struct Accept(pub Payload);

pub struct Commit(pub Payload);

pub struct PreAcceptOK(pub Payload);

pub struct AcceptOK(pub AcceptOKPayload);

pub struct TryPreAccept(pub TryPreAcceptPayload);

pub enum Path {
    Slow(Payload),
    Fast(Payload),
}

#[derive(Default, Clone)]
pub struct CommandLeaderBookKeeping {
    pub client_proposals: Vec<ProposePayload>,
    pub max_recv_ballot: u32,
    pub prepare_oks: u32,
    pub all_equal: bool,
    pub pre_accept_oks: u32,
    pub accept_oks: u32,
    pub nacks: u32,
    pub original_deps: Vec<Instance>,
    pub commited_deps: Vec<Instance>,
    pub recovery_insts: Option<RecoveryPayloadEntry>,
    pub preparing: bool,
    pub trying_to_pre_accept: bool,
    pub possible_quorum: Vec<bool>,
    pub tpa_oks: u32,
    pub commit_time: u64,
}

#[derive(Default, Clone)]
pub struct RecoveryPayloadEntry {
    //pub write_req: WriteRequest,
    pub command: Vec<Command>,
    pub state: PayloadState,
    pub seq: u32,
    pub deps: Vec<Instance>,
    pub pre_accept_count: u32,
    pub command_leader_response: bool,
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

pub struct CoreInfo {
    pub n: usize,
    pub id: ReplicaId,
    pub peer_addr_list: Vec<String>,
    pub peers: Vec<grpcio::Client>,
    pub peer_readers: Vec<ProposePayload>,
    pub peer_writers: Vec<ProposePayload>,
    pub alive: BTreeMap<u32, bool>,
    //pub listeners: grpcio::Client,
    pub state: Option<State>,

    pub shutdown: bool,
    pub thrifty: bool,
    pub exec: bool,
    pub beacon: bool,
    pub dreply: bool,
    pub durable: bool,
    pub stable_store: Option<File>,

    pub preferred_peer_order: Vec<ReplicaId>,
    pub ewma: Vec<f64>,
    pub on_client_connect: bool,
}