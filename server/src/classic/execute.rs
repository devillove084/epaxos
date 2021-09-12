use std::cmp::Ordering;
use std::collections::btree_map::Entry::Occupied;
use std::collections::btree_map::Entry::Vacant;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;

use petgraph::graph::node_index as n;
use petgraph::graph::NodeIndex;

use crate::message::Instance;
use crate::message::LogEntry;
use crate::message::State;

#[derive(Default)]
pub struct ExecutorInner {
    pub components: Vec<Vec<NodeIndex>>,
}

#[derive(Default)]
pub struct Executor {
    // real scc generate
    pub inner: Arc<ExecutorInner>,
    // seq -> slot
    pub vertices: BTreeMap<usize, Vec<usize>>,
    // for sort scc
    pub seq_slot: BTreeMap<usize, (Instance, usize)>,
    // // for judge has executed
    pub cmds: HashMap<usize, LogEntry>,

    // real graph
    pub graph: Vec<(usize, usize)>,

    pub execute_id: Instance,
}

impl ExecutorInner {
    pub fn strong_connect(&self, deps: &Vec<(usize, usize)>) -> Vec<Vec<NodeIndex>> {
        // We need to know how many nodes we have.
        let mut count_hs = HashSet::new();
        for (from, to) in deps.iter() {
            count_hs.insert(from);
            count_hs.insert(to);
        }

        // Construct the graph with number of node,
        // and the weight init 0.
        let mut gr = petgraph::Graph::new();
        for _ in 0..count_hs.len() {
            // This func not add node value, but for node weight.
            gr.add_node(0);
        }
        for (from, to) in deps.iter() {
            // This func add a edge from a to b;
            gr.add_edge(n(*from), n(*to), ());
        }

        // Now, we can get the scc in the graph.
        let mut tarjan_scc = petgraph::algo::TarjanScc::new();
        let mut sccs: Vec<Vec<NodeIndex>> = Vec::new();
        tarjan_scc.run(&gr, |scc| sccs.push(scc.iter().rev().cloned().collect()));

        sccs
    }
}

impl Executor {
    pub fn build_graph(
        &mut self,
        instance: u32,
        gr_map: &mut Vec<(usize, usize)>,
        seq_slot: &mut BTreeMap<usize, (Instance, usize)>,
    ) {
        match self.cmds.get(&(instance as usize)) {
            Some(l) => {
                let log_entry = l.clone();
                if log_entry.deps.is_empty() || l.state == State::Executed {
                    return;
                }
                for to_inst in log_entry.deps.iter() {
                    gr_map.push((instance as usize, *to_inst as usize));
                    seq_slot.insert(instance as usize, (instance, log_entry.seq as usize));
                    self.build_graph(*to_inst, gr_map, seq_slot);
                }
            }
            None => return,
        }
    }

    pub fn execute(&mut self) {
        let local_scc = self.inner.clone();
        let comps = local_scc.strong_connect(&self.graph);
        for comp in comps {
            self.execute_scc(&mut comp.clone());
        }
    }

    pub fn execute_scc(&mut self, comp: &mut Vec<NodeIndex>) {
        comp.sort_by(|a: &NodeIndex, b: &NodeIndex| -> Ordering {
            let seq1 = self.seq_slot.get(&a.index()).unwrap();
            let seq2 = self.seq_slot.get(&b.index()).unwrap();
            if seq1.1 < seq2.1 {
                return Ordering::Less;
            } else if seq1.0.replica < seq2.0.replica {
                return Ordering::Less;
            } else {
                return Ordering::Greater;
            }
        });

        for v in comp {
            //TODO: Write the executed node in log and in-memory db

            // Delete the entry in vertices
            match self.vertices.entry(v.index()) {
                Occupied(o) => {
                    if o.get().is_empty() {
                        o.remove_entry();
                    }
                }
                Vacant(_) => unreachable!(),
            }
        }
    }

    // pub fn has_executed(&self, deps: &Vec<usize>) -> bool {
    //     for v in deps {
    //         let log = self.cmds.get(&v).unwrap();
    //         if log.state == State::Executed {
    //             return true;
    //         }
    //     }
    //     false
    // }
}
