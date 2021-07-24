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

const METHOD_EPAXOS_SERVICE_PRE_ACCEPT: ::grpcio::Method<super::epaxos::Payload, super::epaxos::Payload> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/epaxos.EpaxosService/pre_accept",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_EPAXOS_SERVICE_ACCEPT: ::grpcio::Method<super::epaxos::Payload, super::epaxos::AcceptOKPayload> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/epaxos.EpaxosService/accept",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_EPAXOS_SERVICE_COMMIT: ::grpcio::Method<super::epaxos::Payload, super::epaxos::Empty> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/epaxos.EpaxosService/commit",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_EPAXOS_SERVICE_WRITE: ::grpcio::Method<super::epaxos::WriteRequest, super::epaxos::WriteResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/epaxos.EpaxosService/write",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_EPAXOS_SERVICE_READ: ::grpcio::Method<super::epaxos::ReadRequest, super::epaxos::ReadResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/epaxos.EpaxosService/read",
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

    pub fn pre_accept_opt(&self, req: &super::epaxos::Payload, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::epaxos::Payload> {
        self.client.unary_call(&METHOD_EPAXOS_SERVICE_PRE_ACCEPT, req, opt)
    }

    pub fn pre_accept(&self, req: &super::epaxos::Payload) -> ::grpcio::Result<super::epaxos::Payload> {
        self.pre_accept_opt(req, ::grpcio::CallOption::default())
    }

    pub fn pre_accept_async_opt(&self, req: &super::epaxos::Payload, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::epaxos::Payload>> {
        self.client.unary_call_async(&METHOD_EPAXOS_SERVICE_PRE_ACCEPT, req, opt)
    }

    pub fn pre_accept_async(&self, req: &super::epaxos::Payload) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::epaxos::Payload>> {
        self.pre_accept_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn accept_opt(&self, req: &super::epaxos::Payload, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::epaxos::AcceptOKPayload> {
        self.client.unary_call(&METHOD_EPAXOS_SERVICE_ACCEPT, req, opt)
    }

    pub fn accept(&self, req: &super::epaxos::Payload) -> ::grpcio::Result<super::epaxos::AcceptOKPayload> {
        self.accept_opt(req, ::grpcio::CallOption::default())
    }

    pub fn accept_async_opt(&self, req: &super::epaxos::Payload, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::epaxos::AcceptOKPayload>> {
        self.client.unary_call_async(&METHOD_EPAXOS_SERVICE_ACCEPT, req, opt)
    }

    pub fn accept_async(&self, req: &super::epaxos::Payload) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::epaxos::AcceptOKPayload>> {
        self.accept_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn commit_opt(&self, req: &super::epaxos::Payload, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::epaxos::Empty> {
        self.client.unary_call(&METHOD_EPAXOS_SERVICE_COMMIT, req, opt)
    }

    pub fn commit(&self, req: &super::epaxos::Payload) -> ::grpcio::Result<super::epaxos::Empty> {
        self.commit_opt(req, ::grpcio::CallOption::default())
    }

    pub fn commit_async_opt(&self, req: &super::epaxos::Payload, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::epaxos::Empty>> {
        self.client.unary_call_async(&METHOD_EPAXOS_SERVICE_COMMIT, req, opt)
    }

    pub fn commit_async(&self, req: &super::epaxos::Payload) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::epaxos::Empty>> {
        self.commit_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn write_opt(&self, req: &super::epaxos::WriteRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::epaxos::WriteResponse> {
        self.client.unary_call(&METHOD_EPAXOS_SERVICE_WRITE, req, opt)
    }

    pub fn write(&self, req: &super::epaxos::WriteRequest) -> ::grpcio::Result<super::epaxos::WriteResponse> {
        self.write_opt(req, ::grpcio::CallOption::default())
    }

    pub fn write_async_opt(&self, req: &super::epaxos::WriteRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::epaxos::WriteResponse>> {
        self.client.unary_call_async(&METHOD_EPAXOS_SERVICE_WRITE, req, opt)
    }

    pub fn write_async(&self, req: &super::epaxos::WriteRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::epaxos::WriteResponse>> {
        self.write_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn read_opt(&self, req: &super::epaxos::ReadRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::epaxos::ReadResponse> {
        self.client.unary_call(&METHOD_EPAXOS_SERVICE_READ, req, opt)
    }

    pub fn read(&self, req: &super::epaxos::ReadRequest) -> ::grpcio::Result<super::epaxos::ReadResponse> {
        self.read_opt(req, ::grpcio::CallOption::default())
    }

    pub fn read_async_opt(&self, req: &super::epaxos::ReadRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::epaxos::ReadResponse>> {
        self.client.unary_call_async(&METHOD_EPAXOS_SERVICE_READ, req, opt)
    }

    pub fn read_async(&self, req: &super::epaxos::ReadRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::epaxos::ReadResponse>> {
        self.read_async_opt(req, ::grpcio::CallOption::default())
    }
    pub fn spawn<F>(&self, f: F) where F: ::futures::Future<Output = ()> + Send + 'static {
        self.client.spawn(f)
    }
}

pub trait EpaxosService {
    fn pre_accept(&mut self, ctx: ::grpcio::RpcContext, req: super::epaxos::Payload, sink: ::grpcio::UnarySink<super::epaxos::Payload>);
    fn accept(&mut self, ctx: ::grpcio::RpcContext, req: super::epaxos::Payload, sink: ::grpcio::UnarySink<super::epaxos::AcceptOKPayload>);
    fn commit(&mut self, ctx: ::grpcio::RpcContext, req: super::epaxos::Payload, sink: ::grpcio::UnarySink<super::epaxos::Empty>);
    fn write(&mut self, ctx: ::grpcio::RpcContext, req: super::epaxos::WriteRequest, sink: ::grpcio::UnarySink<super::epaxos::WriteResponse>);
    fn read(&mut self, ctx: ::grpcio::RpcContext, req: super::epaxos::ReadRequest, sink: ::grpcio::UnarySink<super::epaxos::ReadResponse>);
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
    builder = builder.add_unary_handler(&METHOD_EPAXOS_SERVICE_WRITE, move |ctx, req, resp| {
        instance.write(ctx, req, resp)
    });
    let mut instance = s;
    builder = builder.add_unary_handler(&METHOD_EPAXOS_SERVICE_READ, move |ctx, req, resp| {
        instance.read(ctx, req, resp)
    });
    builder.build()
}
