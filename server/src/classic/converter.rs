#![allow(missing_docs)]


use crate::message::*;

use super::epaxos as grpc; // TODO: Change

impl Operation {
    pub fn from_grpc(operation: &grpc::Operation) -> Self {
        match operation {
            grpc::Operation::PUT => Operation{op: 0},
            grpc::Operation::PUT_BLIND => Operation{op: 1},
            grpc::Operation::GET => Operation{op: 2},
        }
    }

    pub fn to_grpc(&self) -> grpc::Operation {
        if self.op == 0 { // TODO: Match change
            grpc::Operation::PUT
        } else if self.op == 1 {
            grpc::Operation::PUT_BLIND
        } else { // 2
            grpc::Operation::GET
        }
    }
}

impl PayloadState {
    pub fn from_grpc(state: &grpc::PayloadState) -> Self {
        match state {
            grpc::PayloadState::PREACCEPTED => PayloadState{state_num: 0},
            grpc::PayloadState::PREACCEPTEDEQ => PayloadState{state_num: 1},
            grpc::PayloadState::ACCEPTED => PayloadState{state_num: 2},
            grpc::PayloadState::COMMITTED => PayloadState{state_num: 3},
            grpc::PayloadState::EXECUTED => PayloadState{state_num: 4},
        }
    }

    pub fn to_grpc(&self) -> grpc::PayloadState {
        if self.state == 0 { // TODO: Change
            grpc::PayloadState::PREACCEPTED
        } else if self.state == 1 {
            grpc::PayloadState::PREACCEPTEDEQ
        } else if self.state == 2 {
            grpc::PayloadState::ACCEPTED
        } else if self.state == 3 {
            grpc::PayloadState::COMMITTED
        } else { // 4
            grpc::PayloadState::EXECUTED
        }
    }
}

impl Command {
    pub fn from_grpc(command: &grpc::Command) -> Self {
        Command {
            op: Operation::from_grpc(&command.get_op()),
            key: command.get_key().to_owned(),
            value: command.get_value(),
        }
    }

    pub fn to_grpc(&self) -> grpc::Command {
        let mut command = grpc::Command::new();
        command.set_op(self.op.to_grpc());
        command.set_key(self.key);
        command.set_value(self.value);
        command
    }
}

impl ProposePayload {
    pub fn from_grpc(propose: &grpc::ProposePayload) -> Self {
        ProposePayload {
            command_id: propose.get_command_id(),
            command: Command::from_grpc(propose.get_command()),
            timestamp: propose.get_timestamp(),
        }
    }

    pub fn to_grpc(&self) -> grpc::ProposePayload {
        let mut propose = grpc::ProposePayload::new();
        propose.set_command_id(self.command_id);
        propose.set_command(self.command.to_grpc());
        propose.set_timestamp(self.timestamp);
        propose
    }
}

impl ProposeReplyPayload {
    pub fn from_grpc(pr: &grpc::ProposeReplyPayload) -> Self {
        ProposeReplyPayload {
            ok: pr.get_ok(),
            command_id: pr.get_command_id(),
            value: pr.get_value(),
            timestamp: pr.get_timestamp(),
        }
    }

    pub fn to_grpc(&self) -> grpc::ProposeReplyPayload {
        let mut pr = grpc::ProposeReplyPayload::new();
        pr.set_ok(self.ok);
        pr.set_timestamp(self.timestamp);
        pr.set_value(self.value);
        pr.set_command_id(self.command_id);
        pr
    }
}

impl PreparePayload {
    pub fn from_grpc(prepare_payload: &grpc::PreparePayload) -> Self {
        PreparePayload {
            leader_id: prepare_payload.get_leader_id(),
            ballot: prepare_payload.get_ballot(),
            instance: Instance::from_grpc(prepare_payload.get_instance()),
        }
    }

    pub fn to_grpc(&self) -> grpc::PreparePayload {
        let mut prepare_payload = grpc::PreparePayload::new();
        prepare_payload.set_ballot(self.ballot);
        prepare_payload.set_leader_id(self.leader_id);
        prepare_payload.set_instance(Instance::to_grpc(&self.instance));
        prepare_payload
    }
}

impl PrepareReplyPayload {
    pub fn from_grpc(prepare_reply_payload: &grpc::PrepareReplyPayload) -> Self {
        PrepareReplyPayload {
            accept_id: prepare_reply_payload.get_accept_id(),
            ok: prepare_reply_payload.get_ok(),
            instance: Instance::from_grpc(prepare_reply_payload.get_instance()),
            ballot: prepare_reply_payload.get_ballot(),
            state: PayloadState::from_grpc(&prepare_reply_payload.get_state()),
            command: prepare_reply_payload.get_command().iter().map(Command::from_grpc).collect(),
            seq:prepare_reply_payload.get_seq(),
            deps: prepare_reply_payload.get_deps().iter().map(Instance::from_grpc).collect(),
        }
    }

    pub fn to_grpc(&self) -> grpc::PrepareReplyPayload {
        let mut prepare_reply_paylaod = grpc::PrepareReplyPayload::new();
        prepare_reply_paylaod.set_accept_id(self.accept_id);
        prepare_reply_paylaod.set_ok(self.ok);
        prepare_reply_paylaod.set_instance(Instance::to_grpc(&self.instance));
        prepare_reply_paylaod.set_ballot(self.ballot);
        prepare_reply_paylaod.set_state(PayloadState::to_grpc(&self.state));
        prepare_reply_paylaod.set_command(protobuf::RepeatedField::from_vec(
            self.command.iter().map(|c| c.to_grpc()).collect(),
        ));
        prepare_reply_paylaod.set_seq(self.seq);
        prepare_reply_paylaod.set_deps(protobuf::RepeatedField::from_vec(
            self.deps.iter().map(|d| d.to_grpc()).collect(),
        ));
        prepare_reply_paylaod
    }
}

impl PreAcceptPayload {
    pub fn from_grpc(preaccept_payload: &grpc::PreAcceptPayload) -> Self {
        PreAcceptPayload {
            leader_id: preaccept_payload.get_leader_id(),
            instance: Instance::from_grpc(preaccept_payload.get_instance()),
            ballot: preaccept_payload.get_ballot(),
            command: preaccept_payload.get_command().iter().map(Command::from_grpc).collect(),
            seq: preaccept_payload.get_seq(),
            deps: preaccept_payload.get_deps().iter().map(Instance::from_grpc).collect(),
        }
    }

    pub fn to_grpc(&self) -> grpc::PreAcceptPayload {
        let mut preaccept_payload = grpc::PreAcceptPayload::new();
        preaccept_payload.set_leader_id(self.leader_id);
        preaccept_payload.set_instance(self.instance.to_grpc());
        preaccept_payload.set_ballot(self.ballot);
        preaccept_payload.set_command(protobuf::RepeatedField::from_vec(
            self.command.iter().map(|c| c.to_grpc()).collect(),
        ));
        preaccept_payload.set_seq(self.seq);
        preaccept_payload.set_deps(protobuf::RepeatedField::from_vec(
            self.deps.iter().map(|dep| dep.to_grpc()).collect(),
        ));
        preaccept_payload
    }
}

impl PreAcceptReplyPayload {
    pub fn from_grpc(preaccept_reply_payload: &grpc::PreAcceptReplyPayload) -> Self {
        PreAcceptReplyPayload {
            instance: Instance::from_grpc(preaccept_reply_payload.get_instance()),
            ok: preaccept_reply_payload.get_ok(),
            ballot: preaccept_reply_payload.get_ballot(),
            command: preaccept_reply_payload.get_command().iter().map(Command::from_grpc).collect(),
            seq: preaccept_reply_payload.get_seq(),
            deps: preaccept_reply_payload.get_deps().iter().map(Instance::from_grpc).collect(),
            commited_deps: preaccept_reply_payload.get_deps().iter().map(Instance::from_grpc).collect(),
        }
    }

    pub fn to_grpc(&self) -> grpc::PreAcceptReplyPayload {
        let mut preaccept_reply_payload = grpc::PreAcceptReplyPayload::new();
        preaccept_reply_payload.set_instance(self.instance.to_grpc());
        preaccept_reply_payload.set_ok(self.ok);
        preaccept_reply_payload.set_ballot(self.ballot);
        preaccept_reply_payload.set_command(protobuf::RepeatedField::from_vec(
            self.command.iter().map(|c| c.to_grpc()).collect(),
        ));
        preaccept_reply_payload.set_deps(protobuf::RepeatedField::from_vec(
            self.deps.iter().map(|dep| dep.to_grpc()).collect(),
        ));
        preaccept_reply_payload.set_committed_deps(protobuf::RepeatedField::from_vec(
            self.commited_deps.iter().map(|dep| dep.to_grpc()).collect(),
        ));
        preaccept_reply_payload
    }
}

impl AcceptPayload {
    pub fn from_grpc(accept_payload: &grpc::AcceptPayload) -> Self {

    }

    pub fn to_grpc(&self) -> grpc::AcceptPayload {

    }
}

impl AcceptReplyPayload {
    pub fn from_grpc(accept_reply_payload: &grpc::AcceptReplyPayload) -> Self {

    }

    pub fn to_grpc(&self) -> grpc::AcceptReplyPayload {

    }
}

impl CommitPayload {
    pub fn from_grpc(commit_payload: &grpc::CommitPayload) -> Self {

    }

    pub fn to_grpc(&self) -> grpc::CommitPayload {

    }
}

impl CommitShortPayload {
    pub fn from_grpc(commit_short_payload: &grpc::CommitShortPayload) -> Self {

    }

    pub fn to_grpc(&self) -> grpc::CommitShortPayload {

    }
}

impl TryPreAcceptPayload {
    pub fn from_grpc(try_pre_accept_payload: &grpc::TryPreAcceptPayload) -> Self {

    }

    pub fn to_grpc(&self) -> grpc::TryPreAcceptPayload {

    }
}

impl TryPreAcceptReplyPayload {
    pub fn from_grpc(try_pre_accept_reply_payload: &grpc::TryPreAcceptReplyPayload) -> Self {

    }

    pub fn to_grpc(&self) -> grpc::TryPreAcceptReplyPayload {

    }
}

// impl RecoveryPayload {
//     pub fn from_grpc(payload: &grpc::RecoveryPayload) -> Self {
//         RecoveryPayload {
//             ballot: payload.get_ballot(),
//             //write_req: WriteRequest::from_grpc(payload.get_write_req()),
//             command: Command::from_grpc(payload.get_command()),
//             seq: payload.get_seq(),
//             deps: payload.get_deps().iter().map(Instance::from_grpc).collect(),
//             instance: Instance::from_grpc(payload.get_instance()),
//             pre_accept_count: payload.get_pre_accept_count(),
//             command_leader_response: payload.get_command_leader_response(),
//         }
//     }

//     pub fn to_grpc(&self) -> grpc::RecoveryPayload {
//         let res = grpc::RecoveryPayload::new();
//         res.set_ballot(self.ballot);
//         res.set_instance(Instance::to_grpc(&self.instance));
//         res.set_deps(protobuf::RepeatedField::from_vec(
//             self.deps.iter().map(|dep| dep.to_grpc()).collect(),
//         ));
//         res.set_seq(self.seq);
//         res.set_command_leader_response(self.command_leader_response);
//         res.set_pre_accept_count(self.pre_accept_count);
//         //res.set_write_req(self.write_req.to_grpc());
//         res.set_command(self.command.to_grpc());
//         res
//     }
// }

// impl CommandLeaderBookKeeping {
//     pub fn from_grpc(info: &grpc::CommandLeaderBookKeeping) -> Self {
//         CommandLeaderBookKeeping {
//             client_proposals: info.get_client_proposal().iter().map(Propose::from_grpc).collect(),
//             max_recv_ballot: info.get_max_recv_ballot(),
//             prepare_oks: info.get_prepare_oks(),
//             all_equal: info.get_all_equal(),
//             pre_accept_oks: info.get_pre_accept_oks(),
//             accept_oks: info.get_accept_oks(),
//             nacks: info.get_nacks(),
//             original_deps: info
//                 .get_original_deps()
//                 .iter()
//                 .map(Instance::from_grpc)
//                 .collect(),
//             commited_deps: info
//                 .get_committed_deps()
//                 .iter()
//                 .map(Instance::from_grpc)
//                 .collect(),
//             recovery_insts: RecoveryPayload::from_grpc(info.get_recovery_insts()),
//             preparing: info.get_preparing(),
//             trying_to_pre_accept: info.get_trying_to_pre_accept(),
//             possible_quorum: info.get_possible_quorum().iter().map(|a| *a).collect(),
//             tpa_oks: info.get_tpa_oks(),
//             commit_time: info.get_commit_time(),
//         }
//     }

//     pub fn to_grpc(&self) -> grpc::CommandLeaderBookKeeping {
//         let mut res = grpc::CommandLeaderBookKeeping::new();
//         res.set_client_proposal(protobuf::RepeatedField::from_vec(
//             self.client_proposals.iter().map(|p| p.to_grpc()).collect(),
//         ));
//         res.set_max_recv_ballot(self.max_recv_ballot);
//         res.set_prepare_oks(self.prepare_oks);
//         res.set_all_equal(self.all_equal);
//         res.set_pre_accept_oks(self.pre_accept_oks);
//         res.set_accept_oks(self.accept_oks);
//         res.set_nacks(self.nacks);
//         res.set_original_deps(protobuf::RepeatedField::from_vec(
//             self.original_deps.iter().map(|dep| dep.to_grpc()).collect(),
//         ));
//         res.set_committed_deps(protobuf::RepeatedField::from_vec(
//             self.commited_deps.iter().map(|dep| dep.to_grpc()).collect(),
//         ));
//         res.set_recovery_insts(self.recovery_insts.to_grpc());
//         res.set_preparing(self.preparing);
//         res.set_trying_to_pre_accept(self.trying_to_pre_accept);
//         res.set_possible_quorum(self.possible_quorum.iter().map(|f| *f).collect());
//         res.set_tpa_oks(self.tpa_oks);
//         res.set_commit_time(self.commit_time);
//         res
//     }
// }

// impl Payload {
//     pub fn from_grpc(payload: &grpc::Payload) -> Self {
//         Payload {
//             ballot: payload.get_ballot(),
//             //command: Command::from_grpc(payload.get_command()),
//             //write_req: WriteRequest::from_grpc(payload.get_write_req()),
//             command: payload.get_command().iter().map(Command::from_grpc).collect(),
//             seq: payload.get_seq(),
//             deps: payload.get_deps().iter().map(Instance::from_grpc).collect(),
//             instance: Instance::from_grpc(payload.get_instance()),
//             state: PayloadState::from_grpc(&payload.get_state()),
//             from_leader: CommandLeaderBookKeeping::from_grpc(payload.get_clbk()),
//         }
//     }

//     pub fn to_grpc(&self) -> grpc::Payload {
//         let mut payload = grpc::Payload::new();
//         //payload.set_write_req(self.write_req.to_grpc());
//         //payload.set_command(self.command.to_grpc());
//         payload.set_command(protobuf::RepeatedField::from_vec(
//             self.command.iter().map(|command| command.to_grpc()).collect(),
//         ));
//         payload.set_seq(self.seq);
//         payload.set_deps(protobuf::RepeatedField::from_vec(
//             self.deps.iter().map(|dep| dep.to_grpc()).collect(),
//         ));
//         payload.set_instance(Instance::to_grpc(&self.instance));
//         payload.set_clbk(self.from_leader.to_grpc());
//         payload
//     }
// }

// impl AcceptOKPayload {
//     pub fn from_grpc(payload: &grpc::AcceptOKPayload) -> Self {
//         AcceptOKPayload {
//             //write_req: WriteRequest::from_grpc(payload.get_command()),
//             command: Command::from_grpc(payload.get_command()),
//             instance: Instance::from_grpc(payload.get_instance()),
//         }
//     }

//     pub fn to_grpc(&self) -> grpc::AcceptOKPayload {
//         let mut payload = grpc::AcceptOKPayload::new();
//         //payload.set_command(self.write_req.to_grpc());
//         payload.set_command(self.command.to_grpc());
//         payload.set_instance(self.instance.to_grpc());
//         payload
//     }
// }

// impl Prepare {
//     pub fn from_grpc(prepare: &grpc::Prepare) -> Self {
//         Prepare {
//             ballot: prepare.get_ballot(),
//             instance: Instance::from_grpc(prepare.get_instance()),
//             leader_id: prepare.get_leader_id(),
//         }
//     }

//     pub fn to_grpc(&self) -> grpc::Prepare {
//         let mut prepare = grpc::Prepare::new();
//         prepare.set_ballot(self.ballot);
//         prepare.set_leader_id(self.leader_id);
//         prepare.set_instance(self.instance.to_grpc());
//         prepare
//     }
// }

// impl PrepareReply {
//     pub fn from_grpc(payload: &grpc::PrepareReply) -> Self {
//         PrepareReply {
//             //write_req: WriteRequest::from_grpc(payload.get_command()),
//             command: Command::from_grpc(payload.get_command()),
//             ballot: payload.get_ballot(),
//             instance: Instance::from_grpc(payload.get_instance()),
//             accept_id: payload.get_accept_id(),
//             ok: payload.get_ok(),
//             payload: Payload::from_grpc(payload.get_payload()),
//         }
//     }

//     pub fn to_grpc(&self) -> grpc::PrepareReply {
//         let mut payload = grpc::PrepareReply::new();
//         //payload.set_command(self.write_req.to_grpc());
//         payload.set_command(self.command.to_grpc());
//         payload.set_ballot(self.ballot);
//         payload.set_instance(self.instance.to_grpc());
//         payload.set_accept_id(self.accept_id);
//         payload.set_ok(self.ok);
//         payload.set_payload(self.payload.to_grpc());
//         payload
//     }
// }

impl PreAcceptOKPayload {
    pub fn from_grpc(pre_accept_ok_payload: &grpc::PreAcceptOKPayload) -> Self {
        PreAcceptOKPayload {
            instance: Instance::from_grpc(pre_accept_ok_payload.get_instance()),
        }
    }

    pub fn to_grpc(&self) -> grpc::PreAcceptOKPayload {
        let mut pre_accept_ok_payload = grpc::PreAcceptOKPayload::new();
        pre_accept_ok_payload.set_instance(self.instance.to_grpc());
        pre_accept_ok_payload
    }
}

impl Instance {
    pub fn from_grpc(instance: &grpc::Instance) -> Self {
        Instance {
            replica: instance.get_replica(),
            slot: instance.get_slot(),
        }
    }

    pub fn to_grpc(&self) -> grpc::Instance {
        let mut instance = grpc::Instance::new();
        instance.set_replica(self.replica);
        instance.set_slot(self.slot);
        instance
    }
}
