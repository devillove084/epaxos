use std::collections::HashSet;

use sharedlib::{tarjan::{Graph, Tarjan}, logic::{Instance}};


pub struct Edge {
    pub from: Instance,
    pub to: Instance,
}

pub struct ExecEntry {
    pub edges: Vec<Edge>,
    pub components: Vec<Vec<usize>>
}

#[test]
pub fn execute_easy() {
    let graph = {
        let mut g = Graph::new(8);
        g.add_edge(0, 1);
        g.add_edge(2, 0);
        g.add_edges(5, vec![2, 6]);
        g.add_edge(6, 5);
        g.add_edge(1, 2);
        g.add_edges(3, vec![1, 2, 4]);
        g.add_edges(4, vec![5, 3]);
        g.add_edges(7, vec![4, 7, 6]);
        g
    };
 
    for component in Tarjan::walk(&graph) {
        println!("{:?}", component);
    }
}

#[test]
pub fn make_test_case() {
    let execs: Vec<ExecEntry> = vec![
        ExecEntry{
            edges: vec![
                Edge{from: Instance{replica: 0, slot: 1}, to: Instance{replica: 0, slot: 0}},
                Edge{from: Instance{replica: 0, slot: 0}, to: Instance{replica: 0, slot: 2}},
                Edge{from: Instance{replica: 0, slot: 2}, to: Instance{replica: 0, slot: 1}},
                Edge{from: Instance{replica: 0, slot: 0}, to: Instance{replica: 0, slot: 3}},
                Edge{from: Instance{replica: 0, slot: 3}, to: Instance{replica: 0, slot: 4}},
            ],
            components: vec![
                vec![4],
                vec![3],
                vec![0, 1, 2],
            ],
        },
        ExecEntry{
            edges: vec![
                Edge{from: Instance{replica: 0, slot: 0}, to: Instance{replica: 0, slot: 1}},
                Edge{from: Instance{replica: 0, slot: 1}, to: Instance{replica: 0, slot: 2}},
                Edge{from: Instance{replica: 0, slot: 2}, to: Instance{replica: 0, slot: 3}},
            ],
            components: vec![
                vec![3],
                vec![2],
                vec![1],
                vec![0],
            ],
        },
    ];
    
    for exec in execs {
        let mut hs = HashSet::new();
        for i in exec.edges.iter() {
            hs.insert(i.from.slot);
            hs.insert(i.to.slot);
        }

        let mut g = Graph::new(hs.len());
        
        for e in exec.edges {
            let from = e.from.slot;
            let to = e.to.slot;
            g.add_edge(from as usize, to as usize);
        }
        
        let gen_comps = Tarjan::walk(&g);
        let g_len = gen_comps.len();
        let c_len = exec.components.len();
        if g_len == c_len {
            for i in 0..c_len {
                let g_c_s = &gen_comps[i];
                let c_s = &exec.components[i];
                if g_c_s.len() == c_s.len() {
                    println!("++++");
                    println!("BT is {:?}", *g_c_s);
                    println!("Original v is {:?}", c_s);
                    println!("____");
                }
            }
        }
    }
}