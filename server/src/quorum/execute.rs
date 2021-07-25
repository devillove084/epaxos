use std::{cmp::{self, Ordering}, collections::HashMap};

use crate::logic::Instance;

#[derive(Debug, Clone)]
pub struct TarjanNode {
    instance: Instance,
    seq: u32,
    index: i32,
    low_link: i32,
    on_stack: bool,
    deps: Vec<TarjanNode>,
    dep_insts: Vec<Instance>,
}

impl TarjanNode {
    pub fn new(instance: Instance, seq: u32, dep_insts: Vec<Instance>) -> Self {
        println!("seq is {:?}", seq);
        return TarjanNode {
            instance,
            index: -1,
            low_link: -1,
            on_stack: false,
            deps: Vec::new(),
            seq: seq,
            dep_insts: dep_insts,
        };
    }

    pub fn visited(&mut self) -> bool {
        return self.index >= 0;
    }

    pub fn reset(&mut self) {
        self.index = -1;
        self.low_link = -1;
        self.deps.clear();
        self.on_stack = false;
    }
}

pub type Scc = Vec<TarjanNode>;

pub fn contains(comp: &Scc, w: &TarjanNode) -> bool {
    for v in comp {
        if v.instance == w.instance {
            return true;
        }
    }
    false
}

#[derive(Debug, Clone)]
pub struct Executor {
    pub instance: Instance,
    pub vertices: HashMap<Instance, TarjanNode>,
    pub index: i32,
    pub stack: Vec<TarjanNode>,
    pub components: Vec<Scc>,
}

impl Executor {
    pub fn new(instance: Instance) -> Self {
        return Executor {
            vertices: HashMap::new(),
            index: 0,
            stack: Vec::new(),
            components: Vec::new(),
            instance,
        };
    }

    pub fn make_executor(&mut self, instance: Instance) -> Executor {
        let e = Self::new(instance);
        e
    }

    pub fn add_exec(&mut self, instance: Instance, seq: u32, inst_deps: Vec<Instance>) {
        self.vertices.insert(instance, TarjanNode::new(instance, seq, inst_deps));
    }

    // Real run Scc function.
    pub fn run(&mut self) {
        let comps = self.strong_connect_tarjan().to_owned();
        for comp in comps {
            self.execute_scc(&mut comp.to_vec());
        }
        self.reset();
        
    }

    pub fn strong_connect_tarjan(&mut self) -> &Vec<Scc> {
        for (_, mut v) in self.vertices.clone() { //TODO: Fix this!!
            v.reset();
            for dep_inst in v.dep_insts.iter() {
                if self.vertices.contains_key(&dep_inst) {
                    let n = self.vertices.get(&dep_inst).unwrap();
                    v.deps.push(n.clone());
                }
            }
        }
    
        for (_, mut v) in self.vertices.clone() {
            if !v.visited() {
                self.visit(&mut v);
            }
        }
        return &self.components;
    }

    pub fn visit(&mut self,  node: &mut TarjanNode) {
        node.index = self.index;
        node.low_link = self.index;
        self.index += 1;
        node.on_stack = false;
        self.stack.push(node.clone());
        
        for dep in node.deps.iter_mut() {
            if !dep.visited() {
                self.visit(dep);
                node.low_link = cmp::min(node.low_link, dep.low_link);
            } else if dep.on_stack {
                node.low_link = cmp::min(node.low_link, dep.index);
            }
        }

        if node.low_link == node.index {
            let mut component: Vec<TarjanNode> = Vec::new();
            loop {
                if self.stack.len() == 0 {
                    break;
                }
                let mut w = self.pop();
                w.on_stack = false;
                if w.instance != node.instance {
                    break;
                }
                component.push(w);
            }
            self.components.push(component);
        }
    }

    pub fn push(&mut self, node: TarjanNode) {
        self.stack.push(node);
    }

    pub fn pop(&mut self) -> TarjanNode {
        let res = self.stack.pop();
        match res {
            Some(r) => return r,
            None => {
                //TODO: for test
                panic!("no deps");
            },
        }
    }

    pub fn execute_scc(&mut self, comp: &mut Scc) {
        for v in comp.iter() {
            for dep_node in v.dep_insts.iter() {
                if self.vertices.contains_key(&dep_node) && 
                contains(comp, &*self.vertices.get(dep_node).unwrap()) {
                    continue;
            }
                

                //TODO: Do we need to filter the node in Scc has executed?
            }
        }
        comp.sort_by(sort_scc);

        for v in comp {
            self.real_execute(v);
        }
    }

    fn real_execute(&mut self, node: &mut TarjanNode) {
        //update executed log!!!!
        println!("The executed node is: {:?}", node);
    }

    fn reset(&mut self) {
        self.index = -1;
        self.stack.clear();
        self.components.clear();
    }
}

pub fn sort_scc(node1: &TarjanNode, node2: &TarjanNode) -> Ordering {
    let seq1 = node1.seq;
    let seq2 = node2.seq;
    if seq1 < seq2 {
        return Ordering::Less;
    }
    if node1.instance.replica < node2.instance.replica {
        return Ordering::Less;
    } else if node1.instance.replica > node2.instance.replica{
        return Ordering::Greater;
    } else {
        return Ordering::Equal;
    }
}