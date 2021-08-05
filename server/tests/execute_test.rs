use std::collections::HashSet;

use petgraph::{
    graph::{node_index as n, NodeIndex},
    Graph,
};
use sharedlib::logic::Instance;

struct Edge {
    from: Instance,
    to: Instance,
}

struct SccEntry {
    edges: Vec<Edge>,
    components: Vec<Vec<NodeIndex>>,
}

fn assert_sccs_eq(
    mut res: Vec<Vec<NodeIndex>>,
    mut answer: Vec<Vec<NodeIndex>>,
    scc_order_matters: bool,
) {
    // normalize the result and compare with the answer.
    for scc in &mut res {
        scc.sort();
    }
    for scc in &mut answer {
        scc.sort();
    }
    if !scc_order_matters {
        res.sort();
        answer.sort();
    }
    assert_eq!(res, answer);
}

#[test]
pub fn scc_test() {
    let execs: Vec<SccEntry> = vec![
        SccEntry {
            edges: vec![
                Edge {
                    from: Instance {
                        replica: 0,
                        slot: 1,
                    },
                    to: Instance {
                        replica: 0,
                        slot: 0,
                    },
                },
                Edge {
                    from: Instance {
                        replica: 0,
                        slot: 0,
                    },
                    to: Instance {
                        replica: 0,
                        slot: 2,
                    },
                },
                Edge {
                    from: Instance {
                        replica: 0,
                        slot: 2,
                    },
                    to: Instance {
                        replica: 0,
                        slot: 1,
                    },
                },
                Edge {
                    from: Instance {
                        replica: 0,
                        slot: 0,
                    },
                    to: Instance {
                        replica: 0,
                        slot: 3,
                    },
                },
                Edge {
                    from: Instance {
                        replica: 0,
                        slot: 3,
                    },
                    to: Instance {
                        replica: 0,
                        slot: 4,
                    },
                },
            ],
            components: vec![vec![n(4)], vec![n(3)], vec![n(0), n(1), n(2)]],
        },
        SccEntry {
            edges: vec![
                Edge {
                    from: Instance {
                        replica: 0,
                        slot: 0,
                    },
                    to: Instance {
                        replica: 0,
                        slot: 1,
                    },
                },
                Edge {
                    from: Instance {
                        replica: 0,
                        slot: 1,
                    },
                    to: Instance {
                        replica: 0,
                        slot: 2,
                    },
                },
                Edge {
                    from: Instance {
                        replica: 0,
                        slot: 2,
                    },
                    to: Instance {
                        replica: 0,
                        slot: 3,
                    },
                },
            ],
            components: vec![vec![n(3)], vec![n(2)], vec![n(1)], vec![n(0)]],
        },
    ];

    for exec in execs {
        let mut gr: Graph<usize, usize> = Graph::new();
        let mut hs = HashSet::new();
        for i in exec.edges.iter() {
            hs.insert(i.from.slot);
            hs.insert(i.to.slot);
        }

        for _ in 0..hs.len() {
            gr.add_node(0);
        }

        for edge in exec.edges.iter() {
            gr.add_edge(n(edge.from.slot as usize), n(edge.to.slot as usize), 0);
        }

        let mut tarjan_scc = petgraph::algo::TarjanScc::new();
        let mut result = Vec::new();
        tarjan_scc.run(&gr, |scc| result.push(scc.iter().rev().cloned().collect()));
        assert_sccs_eq(result, exec.components, true);
    }
}

struct ExecNode {
    id: usize,
    deps: Vec<usize>,
}

struct ExecEntry {
    scc: Vec<ExecNode>,
    execution: Vec<usize>,
}

#[test]
pub fn execute_test() {
    let executed_node: Vec<usize> = vec![1, 3];

    let execs: Vec<ExecEntry> = vec![
        ExecEntry {
            scc: vec![ExecNode {
                id: 4,
                deps: vec![],
            }],
            execution: vec![4],
        },
        ExecEntry {
            scc: vec![ExecNode {
                id: 4,
                deps: vec![1, 3],
            }],
            execution: vec![4],
        },
        ExecEntry {
            scc: vec![ExecNode {
                id: 4,
                deps: vec![1, 2, 3],
            }],
            execution: vec![],
        },
        ExecEntry {
            scc: vec![
                ExecNode {
                    id: 4,
                    deps: vec![9],
                },
                ExecNode {
                    id: 9,
                    deps: vec![5],
                },
                ExecNode {
                    id: 5,
                    deps: vec![8],
                },
                ExecNode {
                    id: 8,
                    deps: vec![4],
                },
            ],
            execution: vec![4, 5, 8, 9],
        },
        ExecEntry {
            scc: vec![
                ExecNode {
                    id: 4,
                    deps: vec![9],
                },
                ExecNode {
                    id: 9,
                    deps: vec![1, 5],
                },
                ExecNode {
                    id: 5,
                    deps: vec![3, 8],
                },
                ExecNode {
                    id: 8,
                    deps: vec![1, 4],
                },
            ],
            execution: vec![4, 5, 8, 9],
        },
        ExecEntry {
            scc: vec![
                ExecNode {
                    id: 4,
                    deps: vec![2, 9],
                },
                ExecNode {
                    id: 9,
                    deps: vec![1, 5],
                },
                ExecNode {
                    id: 5,
                    deps: vec![3, 8],
                },
                ExecNode {
                    id: 8,
                    deps: vec![1, 4],
                },
            ],
            execution: vec![],
        },
    ];

    for exec in execs {
        // 获得所有的scc，
        // 判断所有的执行路径是否是executed
    }
}
