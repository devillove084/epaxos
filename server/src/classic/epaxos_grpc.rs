// This file is generated. Do not edit
// @generated

// https://github.com/Manishearth/rust-clippy/issues/702
#![allow(unknown_lints)]
#![allow(clippy::all)]

#![allow(box_pointers)]
#![allow(dead_code)]
#![allow(missing_docs)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(trivial_casts)]
#![allow(unsafe_code)]
#![allow(unused_imports)]
#![allow(unused_results)]

const METHOD_EPAXOS_SERVICE_PRE_ACCEPT: ::grpcio::Method<super::epaxos::PreAcceptPayload, super::epaxos::PreAcceptReplyPayload> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/epaxos.EpaxosService/pre_accept",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_EPAXOS_SERVICE_ACCEPT: ::grpcio::Method<super::epaxos::AcceptPayload, super::epaxos::AcceptReplyPayload> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/epaxos.EpaxosService/accept",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_EPAXOS_SERVICE_COMMIT: ::grpcio::Method<super::epaxos::CommitPayload, super::epaxos::Empty> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/epaxos.EpaxosService/commit",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_EPAXOS_SERVICE_COMMITSHORT: ::grpcio::Method<super::epaxos::CommitShortPayload, super::epaxos::Empty> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/epaxos.EpaxosService/commitshort",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_EPAXOS_SERVICE_PROPOSE: ::grpcio::Method<super::epaxos::ProposePayload, super::epaxos::Empty> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/epaxos.EpaxosService/propose",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_EPAXOS_SERVICE_PREPARE: ::grpcio::Method<super::epaxos::PreparePayload, super::epaxos::PrepareReplyPayload> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/epaxos.EpaxosService/prepare",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_EPAXOS_SERVICE_TRY_PRE_ACCEPT: ::grpcio::Method<super::epaxos::TryPreAcceptPayload, super::epaxos::TryPreAcceptReplyPayload> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/epaxos.EpaxosService/try_pre_accept",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

#[derive(Clone)]
pub struct EpaxosServiceClient {
    client: ::grpcio::Client,
}

impl EpaxosServiceClient {
    pub fn new(channel: ::grpcio::Channel) -> Self {
        EpaxosServiceClient {
            client: ::grpcio::Client::new(channel),
        }
    }

    pub fn pre_accept_opt(&self, req: &super::epaxos::PreAcceptPayload, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::epaxos::PreAcceptReplyPayload> {
        self.client.unary_call(&METHOD_EPAXOS_SERVICE_PRE_ACCEPT, req, opt)
    }

    pub fn pre_accept(&self, req: &super::epaxos::PreAcceptPayload) -> ::grpcio::Result<super::epaxos::PreAcceptReplyPayload> {
        self.pre_accept_opt(req, ::grpcio::CallOption::default())
    }

    pub fn pre_accept_async_opt(&self, req: &super::epaxos::PreAcceptPayload, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::epaxos::PreAcceptReplyPayload>> {
        self.client.unary_call_async(&METHOD_EPAXOS_SERVICE_PRE_ACCEPT, req, opt)
    }

    pub fn pre_accept_async(&self, req: &super::epaxos::PreAcceptPayload) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::epaxos::PreAcceptReplyPayload>> {
        self.pre_accept_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn accept_opt(&self, req: &super::epaxos::AcceptPayload, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::epaxos::AcceptReplyPayload> {
        self.client.unary_call(&METHOD_EPAXOS_SERVICE_ACCEPT, req, opt)
    }

    pub fn accept(&self, req: &super::epaxos::AcceptPayload) -> ::grpcio::Result<super::epaxos::AcceptReplyPayload> {
        self.accept_opt(req, ::grpcio::CallOption::default())
    }

    pub fn accept_async_opt(&self, req: &super::epaxos::AcceptPayload, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::epaxos::AcceptReplyPayload>> {
        self.client.unary_call_async(&METHOD_EPAXOS_SERVICE_ACCEPT, req, opt)
    }

    pub fn accept_async(&self, req: &super::epaxos::AcceptPayload) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::epaxos::AcceptReplyPayload>> {
        self.accept_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn commit_opt(&self, req: &super::epaxos::CommitPayload, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::epaxos::Empty> {
        self.client.unary_call(&METHOD_EPAXOS_SERVICE_COMMIT, req, opt)
    }

    pub fn commit(&self, req: &super::epaxos::CommitPayload) -> ::grpcio::Result<super::epaxos::Empty> {
        self.commit_opt(req, ::grpcio::CallOption::default())
    }

    pub fn commit_async_opt(&self, req: &super::epaxos::CommitPayload, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::epaxos::Empty>> {
        self.client.unary_call_async(&METHOD_EPAXOS_SERVICE_COMMIT, req, opt)
    }

    pub fn commit_async(&self, req: &super::epaxos::CommitPayload) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::epaxos::Empty>> {
        self.commit_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn commitshort_opt(&self, req: &super::epaxos::CommitShortPayload, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::epaxos::Empty> {
        self.client.unary_call(&METHOD_EPAXOS_SERVICE_COMMITSHORT, req, opt)
    }

    pub fn commitshort(&self, req: &super::epaxos::CommitShortPayload) -> ::grpcio::Result<super::epaxos::Empty> {
        self.commitshort_opt(req, ::grpcio::CallOption::default())
    }

    pub fn commitshort_async_opt(&self, req: &super::epaxos::CommitShortPayload, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::epaxos::Empty>> {
        self.client.unary_call_async(&METHOD_EPAXOS_SERVICE_COMMITSHORT, req, opt)
    }

    pub fn commitshort_async(&self, req: &super::epaxos::CommitShortPayload) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::epaxos::Empty>> {
        self.commitshort_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn propose_opt(&self, req: &super::epaxos::ProposePayload, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::epaxos::Empty> {
        self.client.unary_call(&METHOD_EPAXOS_SERVICE_PROPOSE, req, opt)
    }

    pub fn propose(&self, req: &super::epaxos::ProposePayload) -> ::grpcio::Result<super::epaxos::Empty> {
        self.propose_opt(req, ::grpcio::CallOption::default())
    }

    pub fn propose_async_opt(&self, req: &super::epaxos::ProposePayload, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::epaxos::Empty>> {
        self.client.unary_call_async(&METHOD_EPAXOS_SERVICE_PROPOSE, req, opt)
    }

    pub fn propose_async(&self, req: &super::epaxos::ProposePayload) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::epaxos::Empty>> {
        self.propose_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn prepare_opt(&self, req: &super::epaxos::PreparePayload, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::epaxos::PrepareReplyPayload> {
        self.client.unary_call(&METHOD_EPAXOS_SERVICE_PREPARE, req, opt)
    }

    pub fn prepare(&self, req: &super::epaxos::PreparePayload) -> ::grpcio::Result<super::epaxos::PrepareReplyPayload> {
        self.prepare_opt(req, ::grpcio::CallOption::default())
    }

    pub fn prepare_async_opt(&self, req: &super::epaxos::PreparePayload, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::epaxos::PrepareReplyPayload>> {
        self.client.unary_call_async(&METHOD_EPAXOS_SERVICE_PREPARE, req, opt)
    }

    pub fn prepare_async(&self, req: &super::epaxos::PreparePayload) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::epaxos::PrepareReplyPayload>> {
        self.prepare_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn try_pre_accept_opt(&self, req: &super::epaxos::TryPreAcceptPayload, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::epaxos::TryPreAcceptReplyPayload> {
        self.client.unary_call(&METHOD_EPAXOS_SERVICE_TRY_PRE_ACCEPT, req, opt)
    }

    pub fn try_pre_accept(&self, req: &super::epaxos::TryPreAcceptPayload) -> ::grpcio::Result<super::epaxos::TryPreAcceptReplyPayload> {
        self.try_pre_accept_opt(req, ::grpcio::CallOption::default())
    }

    pub fn try_pre_accept_async_opt(&self, req: &super::epaxos::TryPreAcceptPayload, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::epaxos::TryPreAcceptReplyPayload>> {
        self.client.unary_call_async(&METHOD_EPAXOS_SERVICE_TRY_PRE_ACCEPT, req, opt)
    }

    pub fn try_pre_accept_async(&self, req: &super::epaxos::TryPreAcceptPayload) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::epaxos::TryPreAcceptReplyPayload>> {
        self.try_pre_accept_async_opt(req, ::grpcio::CallOption::default())
    }
    pub fn spawn<F>(&self, f: F) where F: ::futures::Future<Output = ()> + Send + 'static {
        self.client.spawn(f)
    }
}

pub trait EpaxosService {
    fn pre_accept(&mut self, ctx: ::grpcio::RpcContext, req: super::epaxos::PreAcceptPayload, sink: ::grpcio::UnarySink<super::epaxos::PreAcceptReplyPayload>);
    fn accept(&mut self, ctx: ::grpcio::RpcContext, req: super::epaxos::AcceptPayload, sink: ::grpcio::UnarySink<super::epaxos::AcceptReplyPayload>);
    fn commit(&mut self, ctx: ::grpcio::RpcContext, req: super::epaxos::CommitPayload, sink: ::grpcio::UnarySink<super::epaxos::Empty>);
    fn commitshort(&mut self, ctx: ::grpcio::RpcContext, req: super::epaxos::CommitShortPayload, sink: ::grpcio::UnarySink<super::epaxos::Empty>);
    fn propose(&mut self, ctx: ::grpcio::RpcContext, req: super::epaxos::ProposePayload, sink: ::grpcio::UnarySink<super::epaxos::Empty>);
    fn prepare(&mut self, ctx: ::grpcio::RpcContext, req: super::epaxos::PreparePayload, sink: ::grpcio::UnarySink<super::epaxos::PrepareReplyPayload>);
    fn try_pre_accept(&mut self, ctx: ::grpcio::RpcContext, req: super::epaxos::TryPreAcceptPayload, sink: ::grpcio::UnarySink<super::epaxos::TryPreAcceptReplyPayload>);
}

pub fn create_epaxos_service<S: EpaxosService + Send + Clone + 'static>(s: S) -> ::grpcio::Service {
    let mut builder = ::grpcio::ServiceBuilder::new();
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_EPAXOS_SERVICE_PRE_ACCEPT, move |ctx, req, resp| {
        instance.pre_accept(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_EPAXOS_SERVICE_ACCEPT, move |ctx, req, resp| {
        instance.accept(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_EPAXOS_SERVICE_COMMIT, move |ctx, req, resp| {
        instance.commit(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_EPAXOS_SERVICE_COMMITSHORT, move |ctx, req, resp| {
        instance.commitshort(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_EPAXOS_SERVICE_PROPOSE, move |ctx, req, resp| {
        instance.propose(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_EPAXOS_SERVICE_PREPARE, move |ctx, req, resp| {
        instance.prepare(ctx, req, resp)
    });
    let mut instance = s;
    builder = builder.add_unary_handler(&METHOD_EPAXOS_SERVICE_TRY_PRE_ACCEPT, move |ctx, req, resp| {
        instance.try_pre_accept(ctx, req, resp)
    });
    builder.build()
}
