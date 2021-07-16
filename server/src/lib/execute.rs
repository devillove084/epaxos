use std::{cmp::{self, Ordering}, collections::HashMap};

use crate::logic::{Instance, LogEntry, State};

#[derive(Debug, Clone)]
pub struct TarjanNode {
    instance: Instance,
    index: i32,
    low_link: i32,
    dep: Vec<Instance>,
    on_stack: bool,
    seq: u32
}

impl TarjanNode {
    pub fn new(inst: Instance, rid: u32, cmd: Vec<HashMap<usize, LogEntry>>) -> Self {
        let log = cmd[rid as usize].get(&(inst.slot as usize)).to_owned();
        let mut dep = Vec::new();
        let mut seq: u32 = 0;
        match &log {
            Some(log) => {
                for i in log.deps.iter() {
                    dep.push(*i);
                }
                seq = log.seq;
            },
            None => {

            },
        }
        return TarjanNode {
            instance: inst,
            index: 0,
            low_link: 0,
            dep: dep.to_vec(),
            on_stack: false,
            seq,
        };
    }

    pub fn get_dep(
        &mut self,
        deps: Vec<Instance>,
        rid: u32,
        cmd: &Vec<HashMap<usize, LogEntry>>,
    ) -> Vec<TarjanNode> {
        let mut tarjan_deps = Vec::new();
        for dep in deps.iter() {
            let t = Self::new(*dep, rid, cmd.to_vec());
            tarjan_deps.push(t);
        }
        return tarjan_deps;
    }

    pub fn visited(&mut self) -> bool {
        return self.index >= 0;
    }
}

pub type Scc = Vec<TarjanNode>;

pub fn contains(comp: &mut Scc, w: TarjanNode) -> bool {
    for v in comp.iter() {
        if v.instance.replica == w.instance.replica && v.instance.slot == w.instance.slot {
            return true;
        }
    }
    false
}

#[derive(Debug, Clone)]
pub struct Executor {
    pub replica_id: u32,
    pub instance: Instance,
    pub vertices: HashMap<usize, Vec<TarjanNode>>,
    pub index: i32,
    pub stack: Vec<TarjanNode>,
    pub components: Vec<Scc>,
    pub cmds: Vec<HashMap<usize, LogEntry>>,
}

impl Executor {
    pub fn new(inst: Instance) -> Self {
        return Executor {
            vertices: HashMap::new(),
            index: 0,
            stack: Vec::new(),
            components: Vec::new(),
            cmds: Vec::new(),
            replica_id: 0,
            instance: inst,
        };
    }

    pub fn make_executor(&mut self, inst: Instance, rid: u32, cmd: Vec<HashMap<usize, LogEntry>>) -> Executor {
        //let mut t = TarjanNode::new(inst, rid, cmd);
        let mut e = Self::new(inst);
        e.cmds = cmd;
        e.replica_id = rid;
        e
    }

    pub fn run(&mut self, node: TarjanNode, node_dep: Vec<TarjanNode>) {
        let comps = self.strong_connect_tarjan(&mut node.clone(), &mut node_dep.clone()).to_owned();
        for comp in comps.iter() {
            self.execute_scc(&mut comp.to_vec());
        }
        self.reset();
    }

    pub fn strong_connect_tarjan(
        &mut self,
        node: &mut TarjanNode,
        node_dep: &mut Vec<TarjanNode>,
    ) -> &Vec<Scc> {
        let dep_map = &self.cmds[self.instance.replica as usize];
        let mut real_dep = Vec::new();
        for n in node_dep.clone().iter_mut() {
            // 遍历目前节点的依赖
            let dep = &dep_map.get(&(node.instance.slot as usize)).unwrap().deps;
            real_dep.append(&mut n.get_dep(dep.to_vec(), self.replica_id, &self.cmds)); // 找到并构造依赖节点的依赖
            for d in real_dep.iter_mut() {
                match self.vertices.get(&(d.instance.slot as usize)) {
                    Some(deps) => {
                        node_dep.append(&mut deps.clone());
                    },
                    None => {
                        continue;
                    },
                }
            }
        }

        //let mut comp = Vec::new();
        for node in node_dep.iter_mut() {
            if !node.visited() {
                self.visit(node);
            }
        }
        return &self.components;
    }

    pub fn visit(&mut self,  node: &mut TarjanNode) {
        let mut deps = self.vertices.get_mut(&(node.instance.slot as usize)).unwrap().to_owned();

        node.index = self.index;
        node.low_link = self.index;
        self.index += 1;
        node.on_stack = false;
        self.stack.push(node.clone());

        
        for dep in deps.iter_mut() {
            if !dep.visited() {
                self.visit(node);
                node.low_link = cmp::min(node.low_link, dep.low_link);
            } else if dep.on_stack {
                node.low_link = cmp::min(node.low_link, dep.index);
            }
        }

        if node.low_link == node.index {
            let mut component: Vec<TarjanNode> = Vec::new();
            loop {
                let mut w = self.pop();
                w.on_stack = false;
                if w.instance.replica != node.instance.replica
                    && w.instance.slot != node.instance.slot
                {
                    break;
                }
                component.push(w.clone());
            }
            self.components.push(component);
        }
    }

    pub fn push(&mut self, node: TarjanNode) {
        self.stack.push(node);
    }

    pub fn pop(&mut self) -> TarjanNode {
        let len = self.stack.len() - 1;
        let node = &self.stack[len].to_owned();
        self.stack.pop();
        //self.stack = stack;
        node.clone()
    }

    pub fn execute_scc(&mut self, comp: &mut Scc) {
        for v in comp.iter() {
            for d in v.dep.iter() {
                match self.vertices.get(&(d.slot as usize))  {
                    Some(deps) => {
                        for i in deps.iter() {
                            if contains(&mut comp.clone(), i.clone()) {
                                continue;
                            }
                        }
                    },
                    None => {
                        println!("Nop!")
                    },
                }

                let log =  self.cmds[d.replica as usize].get(&(d.slot as usize)).unwrap();
                if log.state == State::Executed {
                    return;
                }
            }
        }
        comp.sort_by(sort_scc);
        for v in comp.iter() {
            self.real_execute(v.clone());
        }
    }
    pub fn real_execute(&mut self, node: TarjanNode) {
        println!("Do the real node{}", node.seq);
    }

    fn reset(&mut self) {
        self.index = 0;
        self.stack.clear();
        self.components.clear();
    }
}

pub fn sort_scc(node1: &TarjanNode, node2: &TarjanNode) -> Ordering {
    let seq1 = node1.seq;
    let seq2 = node2.seq;
    if seq1 < seq2 {
        return Ordering::Less;
    } else if seq1 > seq2 {
        return Ordering::Greater;
    } else {
        return Ordering::Equal;
    }
}