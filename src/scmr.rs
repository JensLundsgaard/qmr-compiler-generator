use std::collections::{HashMap, HashSet};

use petgraph::{graph::NodeIndex, Graph};
use serde::Serialize;

use crate::{backend::solve, structures::*, utils::*};
#[derive(Debug, Serialize)]
pub struct ScmrArchitecture {
    pub width: usize,
    pub height: usize,
    pub alg_qubits: Vec<Location>,
    pub magic_state_qubits: Vec<Location>,
}

impl Architecture for ScmrArchitecture {
    fn locations(&self) -> Vec<Location> {
        return self.alg_qubits.clone();
    }

    fn graph(&self) -> (Graph<Location, ()>, HashMap<Location, NodeIndex>) {
        return self.get_graph();
    }
}
impl ScmrArchitecture {
    fn get_graph(&self) -> (Graph<Location, ()>, HashMap<Location, NodeIndex>) {
        let mut g = Graph::new();
        let mut index_map = HashMap::new();
        for i in 0..self.height {
            for j in 0..self.width {
                let loc = Location::new(i * self.width + j);
                let v = g.add_node(loc);
                index_map.insert(loc, v);
            }
        }
        for i in 0..self.height {
            for j in 0..self.width {
                // edge to above
                if i > 0 {
                    let v1 = index_map[&Location::new(i * self.width + j)];
                    let v2 = index_map[&Location::new((i - 1) * self.width + j)];
                    g.add_edge(v1, v2, ());
                    g.add_edge(v2, v1, ());
                }
                // edge to below
                if i < self.height - 1 {
                    let v1 = index_map[&Location::new(i * self.width + j)];
                    let v2 = index_map[&Location::new((i + 1) * self.width + j)];
                    g.add_edge(v1, v2, ());
                    g.add_edge(v2, v1, ());
                }
                // edge to left
                if j > 0 {
                    let v1 = index_map[&Location::new(i * self.width + j)];
                    let v2 = index_map[&Location::new(i * self.width + j - 1)];
                    g.add_edge(v1, v2, ());
                    g.add_edge(v2, v1, ());
                }
                // edge to right
                if j < self.width - 1 {
                    let v1 = index_map[&Location::new(i * self.width + j)];
                    let v2 = index_map[&Location::new(i * self.width + j + 1)];
                    g.add_edge(v1, v2, ());
                    g.add_edge(v2, v1, ());
                }
            }
        }
        return (g, index_map);
    }
}
#[derive(Debug, Serialize, Clone, Hash, PartialEq, Eq)]
pub struct ScmrGateImplementation {
    path: Vec<Location>,
}
impl GateImplementation for ScmrGateImplementation {}

type ScmrStep = Step<ScmrGateImplementation>;
#[derive(Debug)]
struct IdTransition;
impl Transition<ScmrGateImplementation> for IdTransition {
    fn apply(&self, step: &ScmrStep) -> ScmrStep {
        return ScmrStep {
            implemented_gates: HashSet::new(),
            map: step.map.clone(),
        };
    }
    fn repr(&self) -> String {
        return "id".to_string();
    }

    fn cost(&self) -> f64 {
        0.0
    }
}

fn scmr_transitions(_step: &ScmrStep) -> Vec<IdTransition> {
    return vec![IdTransition];
}

fn scmr_step_cost(_step: &ScmrStep, _arch: &ScmrArchitecture) -> f64 {
    return 1.0;
}

fn scmr_implement_gate(
    step: &ScmrStep,
    arch: &ScmrArchitecture,
    gate: &Gate,
) -> Option<ScmrGateImplementation> {
    let (mut graph, mut loc_to_node) = arch.get_graph();
    for loc in &arch.magic_state_qubits {
        assert!(!arch.alg_qubits.clone().into_iter().any(|l| l == *loc));
        let old_last = graph[graph.node_indices().last().unwrap()];
        graph.remove_node(loc_to_node[loc]);
        loc_to_node.insert(old_last, loc_to_node[loc]);
        loc_to_node.remove(loc);
    }
    for loc in step.map.values().into_iter() {
        let old_last = graph[graph.node_indices().last().unwrap()];
        graph.remove_node(loc_to_node[loc]);
        loc_to_node.insert(old_last, loc_to_node[loc]);
        loc_to_node.remove(loc);
    }
    for loc in step
        .implemented_gates
        .iter()
        .map(|x| x.implementation.path.clone())
        .flatten()
    {
        let old_last = graph[graph.node_indices().last().unwrap()];
        graph.remove_node(loc_to_node[&loc]);
        loc_to_node.insert(old_last, loc_to_node[&loc]);
        loc_to_node.remove(&loc);
    }
    let (starts, ends) = match &gate.gate_type {
        GateType::CX => {
            let (cpos, tpos) = (step.map[&gate.qubits[0]], step.map[&gate.qubits[1]]);
            (
                vertical_neighbors(cpos, arch.width, arch.height),
                horizontal_neighbors(tpos, arch.width),
            )
        }
        GateType::T => {
            let pos = step.map[&gate.qubits[0]];
            let target_neighbors = vertical_neighbors(pos, arch.width, arch.height);
            let msf_neighors = arch
                .magic_state_qubits
                .clone()
                .into_iter()
                .map(|m| horizontal_neighbors(m, arch.width))
                .flatten()
                .collect();
            (target_neighbors, msf_neighors)
        }
    };
    let mut best: Option<(i32, Vec<NodeIndex>)> = None;

    for start in &starts {
        for end in &ends {
            if loc_to_node.contains_key(start) && loc_to_node.contains_key(end) {
                let res = petgraph::algo::astar(
                    &graph,
                    loc_to_node[&start],
                    |finish| finish == loc_to_node[&end],
                    |_e| 1,
                    |_| 0,
                );
                if best.is_none()
                    || ((&res).is_some() && &res.as_ref().unwrap().0 < &best.as_ref().unwrap().0)
                {
                    best = res;
                }
            }
        }
    }
    return best.map(|(_cost, path)| ScmrGateImplementation {
        path: path.into_iter().map(|n| graph[n]).collect(),
    });
}

pub fn scmr_solve(c: &Circuit, a: &ScmrArchitecture) -> CompilerResult<ScmrGateImplementation> {
    return solve(
        c,
        a,
        &scmr_transitions,
        scmr_implement_gate,
        scmr_step_cost,
        None,
    );
}
