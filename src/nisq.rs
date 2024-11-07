use crate::backend::solve;
use crate::utils::{Architecture, Circuit, Location, Qubit, Step, Transition};
use petgraph::{graph::NodeIndex, Graph};
use std::collections::HashMap;

pub struct NisqArchitecture {
    graph: Graph<Location, ()>,
    index_map: HashMap<Location, NodeIndex>,
}
impl NisqArchitecture {
    pub fn new(graph: Graph<Location, ()>) -> Self {
        let mut index_map = HashMap::new();
        for ind in graph.node_indices() {
            index_map.insert(graph[ind], ind);
        }
        return NisqArchitecture { graph, index_map };
    }
}

impl Architecture for NisqArchitecture {
    fn get_graph(&self) -> &Graph<Location, ()> {
        return &self.graph;
    }

    fn get_locations(&self) -> Vec<&Location> {
        let mut locations = Vec::new();
        for node in self.graph.node_indices() {
            locations.push(&self.graph[node]);
        }
        return locations;
    }
}

fn swap_on_edge(
    map: &HashMap<Qubit, Location>,
    locs: (Location, Location),
) -> HashMap<Qubit, Location> {
    let mut new_map = map.clone();
    for (qubit, loc) in map {
        if loc == &locs.0 {
            new_map.insert(*qubit, locs.1);
        } else if loc == &locs.1 {
            new_map.insert(*qubit, locs.0);
        }
    }
    return new_map;
}

struct NisqTrans {
    edge: (Location, Location),
}

impl Transition for NisqTrans {
    fn apply(&self, step: &Step) -> Step {
        let mut new_step = step.clone();
        new_step.map = swap_on_edge(&step.map, self.edge);
        new_step.gates = Vec::new();
        return new_step;
    }
    fn repr(&self) -> String {
        return format!("swap {:?} {:?}", self.edge.0, self.edge.1);
    }

    fn cost(&self) -> f64 {
        if self.edge.0 == self.edge.1 {
            0.0
        } else {
            1.0
        }
    }
}

fn nisq_transitions(arch: &NisqArchitecture) -> Vec<NisqTrans> {
    let mut transitions = Vec::new();
    transitions.push(NisqTrans {
        edge: (Location(0), Location(0)),
    });
    for edge in arch.graph.edge_indices() {
        let (source, target) = arch.graph.edge_endpoints(edge).unwrap();
        let (loc1, loc2) = (arch.graph[source], arch.graph[target]);
        let trans = NisqTrans { edge: (loc1, loc2) };
        transitions.push(trans);
    }
    return transitions;
}

fn nisq_step_valid(step: &Step, arch: &NisqArchitecture) -> bool {
    let graph = arch.get_graph();
    for gate in &step.gates {
        let (cpos, tpos) = (step.map.get(&gate.qubits[0]), step.map.get(&gate.qubits[1]));
        match cpos {
            Some(cpos) => match tpos {
                Some(tpos) => {
                    if !(graph.contains_edge(arch.index_map[cpos], arch.index_map[tpos])
                        || graph.contains_edge(arch.index_map[tpos], arch.index_map[cpos]))
                    {
                        return false;
                    }
                }
                None => return false,
            },
            None => return false,
        }
    }
    return true;
}

fn nisq_step_cost(_step: &Step) -> f64 {
    0.0
}

pub fn nisq_solve(c: &Circuit, a: &NisqArchitecture) -> (Vec<Step>, Vec<String>, f64) {
    solve(c, a, &nisq_transitions(a), nisq_step_valid, nisq_step_cost)
}
