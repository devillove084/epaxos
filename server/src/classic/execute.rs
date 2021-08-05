use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;

use petgraph::graph::node_index as n;
use petgraph::graph::NodeIndex;

use crate::logic::LogEntry;

#[derive(Debug, Default)]
pub struct WriteEntry {
    pub key: String,
    pub value: i32,
    pub seq: u32,
}

#[derive(Default)]
pub struct ExecutorInner {
    pub components: Vec<Vec<NodeIndex>>,
}

#[derive(Default)]
pub struct Executor {
    pub inner: Arc<ExecutorInner>,
    pub deps: Vec<(usize, usize)>,
    pub replica_id: u32,
    pub cmd: HashMap<usize, LogEntry>,
}

impl ExecutorInner {
    pub fn strong_connect(&self, deps: &Vec<(usize, usize)>) -> Vec<Vec<NodeIndex>> {
        let mut count_hs = HashSet::new();
        for (from, to) in deps.iter() {
            count_hs.insert(from);
            count_hs.insert(to);
        }

        let mut gr = petgraph::Graph::new();
        for _ in 0..count_hs.len() {
            // This func not add node value, but for node weight.
            gr.add_node(0);
        }

        for (from, to) in deps.iter() {
            // This func add a edge from a to b;
            gr.add_edge(n(*from), n(*to), ());
        }
        let mut tarjan_scc = petgraph::algo::TarjanScc::new();
        let mut sccs: Vec<Vec<NodeIndex>> = Vec::new();
        tarjan_scc.run(&gr, |scc| sccs.push(scc.iter().rev().cloned().collect()));

        sccs
    }

    pub fn execute_scc(&self, comp: Vec<NodeIndex>, cmd: HashMap<usize, LogEntry>) -> bool {
        let mut writebatch: Vec<WriteEntry> = Vec::new();
        let mut seqs = Vec::new();
        let mut map = HashMap::new();
        for node in comp {
            let seq = cmd[&node.index()].seq;
            seqs.push(seq);
            map.insert(seq, node);
        }
        seqs.sort();

        for s in seqs {
            let slot = map.get(&s).unwrap().index();
            let log_entry = &cmd[&slot];

            let w = WriteEntry {
                key: log_entry.key.clone(),
                value: log_entry.value,
                seq: s,
            };

            writebatch.push(w);
        }
        self.execute(writebatch)
    }

    pub fn execute(&self, _: Vec<WriteEntry>) -> bool {
        // TODO: Write the finnal
        println!("Just write the execute seq key && value into embed store!!!");
        true
    }
}

impl Executor {
    pub fn execute(&self) -> bool {
        let local_scc = self.inner.clone();
        let comps = local_scc.strong_connect(&self.deps);
        let mut flag = true;
        for comp in comps {
            smol::block_on(async {
                flag &= local_scc.execute_scc(comp, self.cmd.clone());
            });
        }

        flag
    }
}
