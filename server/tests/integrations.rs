use std::collections::HashMap;
use std::str::from_utf8;
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;

use sharedlib::execute::*;
use sharedlib::logic::{Instance, LogEntry, State};

#[test]
fn mock_execute() {
    let mut slots = Vec::new();
    for _ in 0..100 {
        slots.push(thread_rng().gen_range(1..10));
    }
    let cmd = mock_random_cmd(slots.clone());
    // for map in &cmd {
    //     for i  in slots.iter() {
    //         println!("VVVVVV is {:?}", map.get(&(*i as usize)));
    //     }
    // }
    for i in 0..10 {
        let s = thread_rng().gen_range(1..10);
        let inst = Instance{replica: i, slot: s};
        let node = TarjanNode::new(inst, i, cmd.clone());
        let deps_inst = &cmd[i as usize].get(&(s as usize)).unwrap().deps;
        let mut node_dep = Vec::new();
        for ist in deps_inst {
            node_dep.push(TarjanNode::new(*ist,i, cmd.clone()));
        }
        let mut e = Executor::new(Instance{replica: i, slot: s});
        let mut ke = e.make_executor(inst, i, cmd.clone());
        ke.run(node, node_dep);
        println!("hahahahhahahahahhah");
    }
    
}

pub fn mock_random_cmd(slots: Vec<u32>) -> Vec<HashMap<usize, LogEntry>> {
    let key: Vec<u8> = thread_rng().sample_iter(&Alphanumeric).take(2).collect();
    let value: i32 = thread_rng().gen();
    let mut cmd = Vec::new();
    let mut log_hash_map = HashMap::new();
    
    for i in 1..100 {
        let seq = 100 + i;
        let mut deps = Vec::new();
        for _ in 0..100 {
            deps.push(Instance{replica: i, slot: thread_rng().gen_range(1..10)});
        }
        let log = LogEntry {
            key: from_utf8(&key).unwrap().to_string(),
            value,
            seq,
            deps,
            state: State::Committed,
        };

        for k in slots.iter() {
            log_hash_map.insert(*k as usize, log.clone());
        }
    
        cmd.push(log_hash_map.clone());
    }
    cmd
}