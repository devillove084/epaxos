// use std::env;

use std::env;

use smol::fs::File;
// use grpcio::Server;
use yaml_rust::{YamlLoader, YamlEmitter};
use smol::io::AsyncReadExt;

#[derive(Default, Clone)]
pub struct CLIParam {
    pub replica_id: u32,
    //pub peer_addr_list: Vec<String>,
    pub thrifty: bool,
    pub exec: bool,
    pub dreply: bool,
    pub beacon: bool,
    pub durable: bool,
}



impl CLIParam {
    pub fn get_param(&self) -> Option<Self> {
        let args: Vec<String> = env::args().collect();
        let id: u32 = args[1].parse().unwrap();
        let master_addr: String = args[2].parse().unwrap();
        let master_port: u16 = args[3].parse().unwrap();
        let port_num: u16 = args[4].parse().unwrap();
        let thrifty: bool = args[5].parse().unwrap();
        let exec: bool = args[6].parse().unwrap();
        let dreply: bool = args[7].parse().unwrap();
        let beacon: bool = args[8].parse().unwrap();
        let durable: bool = args[9].parse().unwrap();

        //let (replica_id, node_list) = register_with_command_leader(master_addr.to_string(), master_port);

        Some(CLIParam {
            replica_id: id,
            //peer_addr_list: node_list.to_vec(),
            thrifty,
            exec,
            dreply,
            beacon,
            durable,
        })
    }

    pub async fn load_file(&self, filepath: String) {
        let file = File::open(filepath).await;
        match file {
            Ok(mut f) => {
                let mut contents = Vec::new();
                let result = f.read_to_end(&mut contents).await;
                if result.is_err() {
                    panic!("Read File is wrong!");
                }
                let ss = std::str::from_utf8(&contents).unwrap();
                println!("Result is {}", ss);
                // let docs = YamlLoader::from(f);
                // let sth = docs
            },
            Err(e) => {
                panic!("File is something wrong{:?}", e);
            },
        }
        //let docs = YamlLoader::load_from_str(source)
    }
}

// pub fn register_with_command_leader(addr: String, port: u16) -> (u32, &'static Vec<String>) {
//     let mut args = RegisterArgs::new(addr, port);
//     let mut reply = RegisterReply::new();

    
// }

#[cfg(test)]
mod test {
    use super::CLIParam;

    #[test]
    fn test_cli() {
        let ccc = CLIParam::default();
        smol::block_on(async {
            ccc.load_file("/Users/blade/Documents/Repos/epaxos/doc/example_epaxos.yml".to_string()).await;

        });
    }
}
