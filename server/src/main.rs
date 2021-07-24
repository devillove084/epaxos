use std::{env, sync::Arc, thread};
use grpcio::Environment;
use sharedlib::{epaxos_grpc::create_epaxos_service, logic::ReplicaId, server::EpaxosServerImpl};


fn main() {
    let args: Vec<String> = env::args().collect();
    let ip = &args[1];
    let port: u16= args[2].parse().unwrap();
    let id: u32 = args[3].parse().unwrap();
    let r1: u32 = args[4].parse().unwrap();
    let r2: u32 = args[5].parse().unwrap();

    let nn = EpaxosServerImpl::init(ReplicaId(id), vec![ReplicaId(r1), ReplicaId(r2)],);
    let service = create_epaxos_service(nn);
    let mut server = grpcio::ServerBuilder::new(Arc::new(Environment::new(1)))
                                    .register_service(service)
                                    .bind(ip, port)
                                    .build()
                                    .expect("Failed to build epaxos server");

    server.start();
    println!("Server start!!!");
    // Blocks the main thread forever
    loop {
        thread::park();
    }
}