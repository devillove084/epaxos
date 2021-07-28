use log::info;
use protoc_grpcio;

fn main() {
    let proto_root = ".";
    let output = "src/quorum";
    info!("cargo:rerun-if-changed={}", proto_root);
    protoc_grpcio::compile_grpc_protos(&["epaxos.proto"], &[proto_root], &output, None)
        .expect("Failed to compile gRPC definitions!");
}
