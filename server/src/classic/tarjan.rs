use std::collections::{BTreeMap, BTreeSet};
 
#[derive(Clone, Debug)]
pub struct Graph {
    pub neighbors: BTreeMap<usize, BTreeSet<usize>>,
}
 
impl Graph {
    pub fn new(size: usize) -> Self {
        Self {
            neighbors: (0..size).fold(BTreeMap::new(), |mut acc, x| {
                acc.insert(x, BTreeSet::new());
                acc
            }),
        }
    }
 
    pub fn edges<'a>(&'a self, vertex: usize) -> impl Iterator<Item = usize> + 'a {
        self.neighbors[&vertex].iter().cloned()
    }
 
    pub fn add_edge(&mut self, from: usize, to: usize) {
        assert!(to < self.len());
        self.neighbors.get_mut(&from).unwrap().insert(to);
    }
 
    pub fn add_edges(&mut self, from: usize, to: impl IntoIterator<Item = usize>) {
        let limit = self.len();
 
        self.neighbors
            .get_mut(&from)
            .unwrap()
            .extend(to.into_iter().filter(|x| {
                assert!(*x < limit);
                true
            }));
    }
 
    pub fn is_empty(&self) -> bool {
        self.neighbors.is_empty()
    }
 
    pub fn len(&self) -> usize {
        self.neighbors.len()
    }
}
 
#[derive(Clone)]
struct VertexState {
    index: usize,
    low_link: usize,
    on_stack: bool,
}
 
pub struct Tarjan<'a> {
    graph: &'a Graph,
    index: usize,
    stack: Vec<usize>,
    state: Vec<VertexState>,
    components: Vec<BTreeSet<usize>>,
}
 
impl<'a> Tarjan<'a> {
    const INVALID_INDEX: usize = usize::MAX;
 
    pub fn walk(graph: &'a Graph) -> Vec<BTreeSet<usize>> {
        Self {
            graph,
            index: 0,
            stack: Vec::new(),
            state: vec![
                VertexState {
                    index: Self::INVALID_INDEX,
                    low_link: Self::INVALID_INDEX,
                    on_stack: false
                };
                graph.len()
            ],
            components: Vec::new(),
        }
        .visit_all()
    }
 
    fn visit_all(mut self) -> Vec<BTreeSet<usize>> {
        for vertex in 0..self.graph.len() {
            if self.state[vertex].index == Self::INVALID_INDEX {
                self.visit(vertex);
            }
        }
 
        self.components
    }
 
    fn visit(&mut self, v: usize) {
        let v_ref = &mut self.state[v];
        v_ref.index = self.index;
        v_ref.low_link = self.index;
        self.index += 1;
        self.stack.push(v);
        v_ref.on_stack = true;
 
        for w in self.graph.edges(v) {
            let w_ref = &self.state[w];
            if w_ref.index == Self::INVALID_INDEX {
                self.visit(w);
                let w_low_link = self.state[w].low_link;
                let v_ref = &mut self.state[v];
                v_ref.low_link = v_ref.low_link.min(w_low_link);
            } else if w_ref.on_stack {
                let w_index = self.state[w].index;
                let v_ref = &mut self.state[v];
                v_ref.low_link = v_ref.low_link.min(w_index);
            }
        }
 
        let v_ref = &self.state[v];
        if v_ref.low_link == v_ref.index {
            let mut component = BTreeSet::new();
 
            loop {
                let w = self.stack.pop().unwrap();
                self.state[w].on_stack = false;
                component.insert(w);
                if w == v {
                    break;
                }
            }
 
            self.components.push(component);
        }
    }
}