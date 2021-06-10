//use protoc_rust_grpc;
use protoc_grpcio;

fn main() {
    // protoc_rust_grpc::Codegen::new()
    // .out_dir("src/lib")
    // .input("epaxos.proto")
    // .rust_protobuf(true)
    // .run()
    // .expect("error compiling protocol buffer");
    let proto_root = "";
    let output = "src/lib";
    println!("cargo:rerun-if-changed={}", proto_root);
    protoc_grpcio::compile_grpc_protos(
        &["epaxos.proto"],
        &[proto_root],
        &output,
        None
    ).expect("Failed to compile gRPC definitions!");
}
