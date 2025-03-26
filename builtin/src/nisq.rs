use petgraph::{graph::NodeIndex, Graph};
use serde::Serialize;
use solver::backend::{sabre_solve, solve};
use solver::structures::*;
use std::collections::{HashMap, HashSet};

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
    pub fn get_graph(&self) -> &Graph<Location, ()> {
        return &self.graph;
    }
}

impl Architecture for NisqArchitecture {
    fn locations(&self) -> Vec<Location> {
        let mut locations = Vec::new();
        for node in self.graph.node_indices() {
            locations.push(self.graph[node]);
        }
        return locations;
    }
    fn graph(&self) -> (Graph<Location, ()>, HashMap<Location, NodeIndex>) {
        return (self.graph.clone(), self.index_map.clone());
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
#[derive(Debug)]
struct NisqTrans {
    edge: (Location, Location),
}
#[derive(Clone, Debug, Serialize, Hash, PartialEq, Eq)]
pub struct NisqGateImplementation {
    edge: (Location, Location),
}

impl GateImplementation for NisqGateImplementation {}

type NisqStep = Step<NisqGateImplementation>;

impl Transition<NisqGateImplementation, NisqArchitecture> for NisqTrans {
    fn apply(&self, step: &NisqStep) -> NisqStep {
        let mut new_step = step.clone();
        new_step.map = swap_on_edge(&step.map, self.edge);
        new_step.implemented_gates = HashSet::new();
        return new_step;
    }
    fn repr(&self) -> String {
        return format!("{:?}", self);
    }

    fn cost(&self, _arch : &NisqArchitecture) -> f64 {
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
        edge: (Location::new(0), Location::new(0)),
    });
    for edge in arch.graph.edge_indices() {
        let (source, target) = arch.graph.edge_endpoints(edge).unwrap();
        let (loc1, loc2) = (arch.graph[source], arch.graph[target]);
        let trans = NisqTrans { edge: (loc1, loc2) };
        transitions.push(trans);
    }
    return transitions;
}

fn nisq_implement_gate(
    step: &NisqStep,
    arch: &NisqArchitecture,
    gate: &Gate,
) -> Vec<NisqGateImplementation> {
    let graph = arch.get_graph();
    let (cpos, tpos) = (step.map.get(&gate.qubits[0]), step.map.get(&gate.qubits[1]));
    match (cpos, tpos) {
        (Some(cpos), Some(tpos))
            if graph.contains_edge(arch.index_map[cpos], arch.index_map[tpos]) =>
        {
            vec![NisqGateImplementation {
                edge: (*cpos, *tpos),
            }]
        }
        _ => vec![],
    }
}

fn nisq_step_cost(_step: &NisqStep, _arch: &NisqArchitecture) -> f64 {
    0.0
}

fn mapping_heuristic(arch: &NisqArchitecture, c: &Circuit, map: &HashMap<Qubit, Location>) -> f64 {
    let graph = arch.get_graph();
    let mut cost = 0;
    for gate in &c.gates {
        let (cpos, tpos) = (map.get(&gate.qubits[0]), map.get(&gate.qubits[1]));
        let (cind, tind) = (arch.index_map[cpos.unwrap()], arch.index_map[tpos.unwrap()]);
        let sp_res = petgraph::algo::astar(graph, cind, |n| n == tind, |_| 1, |_| 0);

        match sp_res {
            Some((c, _)) => {cost += c;
                //  println!("gate: {:?}, distance {:?}", gate, c)
                 }
            None => panic!(
                "Disconnected graph. No path found from {:?} to {:?}",
                cpos, tpos
            ),
        }
    }
    return cost as f64;
}


pub fn nisq_solve_sabre(
    c: &Circuit,
    a: &NisqArchitecture,
) -> CompilerResult<NisqGateImplementation> {
    return sabre_solve(
        c,
        a,
        &|_s| nisq_transitions(a),
        nisq_implement_gate,
        nisq_step_cost,
        Some(mapping_heuristic),
        false,
    );
}

pub fn nisq_solve(c: &Circuit, a: &NisqArchitecture) -> CompilerResult<NisqGateImplementation> {
    return solve(
        c,
        a,
        &|_s| nisq_transitions(a),
        nisq_implement_gate,
        nisq_step_cost,
        Some(mapping_heuristic),
        false,
    );
}
