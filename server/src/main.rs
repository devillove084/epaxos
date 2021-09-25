use grpcio::Environment;
use log::info;
use server::{
    classic::{epaxos_grpc::create_epaxos_service, server::EpaxosServerImpl},
    cli::cli::CLIParam,
};
use std::{env, sync::Arc, thread};

fn main() {
    // replicaId, nodeList, *thrifty, *exec, *dreply, *beacon, *durable
    let args: Vec<String> = env::args().collect();
    // let ip = &args[1];
    // let port: u16 = args[2].parse().unwrap();
    // let id: u32 = args[3].parse().unwrap();
    // let r1: u32 = args[4].parse().unwrap();
    // let r2: u32 = args[5].parse().unwrap();

    let master_addr: String = args[1].parse().unwrap();
    let master_port: u16 = args[2].parse().unwrap();
    let id: u32 = args[3].parse().unwrap();
    let port_num: u16 = args[4].parse().unwrap();
    let thrifty: bool = args[5].parse().unwrap();
    let exec: bool = args[6].parse().unwrap();
    let dreply: bool = args[7].parse().unwrap();
    let beacon: bool = args[8].parse().unwrap();
    let durable: bool = args[9].parse().unwrap();

    let r1: u32 = args[10].parse().unwrap();
    let r2: u32 = args[11].parse().unwrap();

    let replica_param = CLIParam {
        replica_id: id,
        //peer_addr_list:
        thrifty,
        exec,
        dreply,
        beacon,
        durable,
    };

    let server_init = EpaxosServerImpl::init(id, vec![r1, r2], replica_param);
    let service = create_epaxos_service(server_init);
    let mut server = grpcio::ServerBuilder::new(Arc::new(Environment::new(2)))
        .register_service(service)
        .bind(master_addr, master_port)
        .build()
        .expect("Failed to build epaxos server");

    server.start();
    info!("Server start!!!");
    // Blocks the main thread forever
    loop {
        thread::park();
    }
}
