use std::collections::{HashMap, HashSet};

use petgraph::{graph::NodeIndex, Graph};
use serde::Serialize;
use solver::{
    backend::{solve, solve_joint_optimize_parallel},
    structures::{
        Architecture, Circuit, CompilerResult, Gate, GateImplementation, Location, Operation,
        QubitMap, Step, Transition,
    },
    utils::{all_paths, horizontal_neighbors, vertical_neighbors},
};

const CODE_DISTANCE: usize = 11;

#[derive(Clone)]
pub struct ILQArch {
    pub stack_depth: usize,
    pub width: usize,
    pub height: usize,
    pub alg_qubits: Vec<Location>,
    pub magic_state_qubits: Vec<Location>,
}

impl Architecture for ILQArch {
    fn locations(&self) -> Vec<Location> {
        return self.alg_qubits.clone();
    }

    fn graph(
        &self,
    ) -> (
        petgraph::Graph<Location, ()>,
        std::collections::HashMap<Location, petgraph::prelude::NodeIndex>,
    ) {
        return self.get_graph();
    }
}

impl ILQArch {
    fn get_graph(&self) -> (Graph<Location, ()>, HashMap<Location, NodeIndex>) {
        let mut g = Graph::new();
        let mut index_map = HashMap::new();
        for i in 0..self.height {
            for j in 0..self.width {
                for k in 0..self.stack_depth {
                    let loc =
                        Location::new(i * self.width * self.stack_depth + j * self.stack_depth + k);
                    let v = g.add_node(loc);
                    index_map.insert(loc, v);
                }
            }
        }
        for i in 0..self.height {
            for j in 0..self.width {
                // edge to above
                if i > 0 {
                    for k1 in 0..self.stack_depth {
                        for k2 in 0..self.stack_depth {
                            let v1 = index_map[&Location::new(
                                i * self.width * self.stack_depth + j * self.stack_depth + k1,
                            )];
                            let v2 = index_map[&Location::new(
                                (i - 1) * self.width * self.stack_depth + j * self.stack_depth + k2,
                            )];
                            g.update_edge(v1, v2, ());
                            g.update_edge(v2, v1, ());
                        }
                    }
                }
                // edge to below
                if i < self.height - 1 {
                    for k1 in 0..self.stack_depth {
                        for k2 in 0..self.stack_depth {
                            let v1 = index_map[&Location::new(
                                i * self.width * self.stack_depth + j * self.stack_depth + k1,
                            )];
                            let v2 = index_map[&Location::new(
                                (i + 1) * self.width * self.stack_depth + j * self.stack_depth + k2,
                            )];
                            g.update_edge(v1, v2, ());
                            g.update_edge(v2, v1, ());
                        }
                    }
                }
                // edge to left
                if j > 0 {
                    for k1 in 0..self.stack_depth {
                        for k2 in 0..self.stack_depth {
                            let v1 = index_map[&Location::new(
                                i * self.width * self.stack_depth + j * self.stack_depth + k1,
                            )];
                            let v2 = index_map[&Location::new(
                                i * self.width * self.stack_depth + (j - 1) * self.stack_depth + k2,
                            )];
                            g.update_edge(v1, v2, ());
                            g.update_edge(v2, v1, ());
                        }
                    }
                }
                // edge to right
                if j < self.width - 1 {
                    for k1 in 0..self.stack_depth {
                        for k2 in 0..self.stack_depth {
                            let v1 = index_map[&Location::new(
                                i * self.width * self.stack_depth + j * self.stack_depth + k1,
                            )];
                            let v2 = index_map[&Location::new(
                                i * self.width * self.stack_depth + (j + 1) * self.stack_depth + k2,
                            )];
                            g.update_edge(v1, v2, ());
                            g.update_edge(v2, v1, ());
                        }
                    }
                }
            }
        }
        return (g, index_map);
    }
}

pub fn compact_layout(alg_qubit_count: usize, stack_depth: usize) -> ILQArch {
    let groups = alg_qubit_count.div_ceil(stack_depth);
    let width = (2 * groups.div_ceil(2)) + 1;
    let height = 5;
    let mut alg_qubits = Vec::new();
    for j in (1..width - 1).step_by(2) {
        for k in 0..stack_depth {
            alg_qubits.push(Location::new(1 * width * stack_depth + j * stack_depth + k));
            alg_qubits.push(Location::new(3 * width * stack_depth + j * stack_depth + k));
        }
    }
    let mut perimeter = Vec::new();
    let top_edge = (0..width).map(|i| Location::new(i));
    let mut top_edge = Vec::new();
    for j in 0..width {
        for k in 0..stack_depth {
            top_edge.push(Location::new(0 * width * stack_depth + j * stack_depth + k));
        }
    }
    let right_edge = (1..height).map(|i| Location::new(i * width + width - 1));
    let mut right_edge = Vec::new();
    for i in 1..height {
        for k in 0..stack_depth {
            right_edge.push(Location::new(
                i * width * stack_depth + (width - 1) * stack_depth + k,
            ));
        }
    }
    let bottom_edge = (0..width - 1)
        .rev()
        .map(|i| Location::new(i + width * (height - 1)));
    let mut bottom_edge = Vec::new();
    for j in (0..width - 1).rev() {
        for k in 0..stack_depth {
            bottom_edge.push(Location::new(
                (height - 1) * width * stack_depth + j * stack_depth + k,
            ));
        }
    }
    let mut left_edge = Vec::new();
    for i in (1..height - 1).rev() {
        for k in 0..stack_depth {
            left_edge.push(Location::new(i * width * stack_depth + k));
        }
    }
    perimeter.extend(top_edge);
    perimeter.extend(right_edge);
    perimeter.extend(bottom_edge);
    perimeter.extend(left_edge);
    // iterate over every other location on the perimeter
    let mut magic_state_qubits = Vec::new();
    for i in (1..perimeter.len()).step_by(2) {
        magic_state_qubits.push(perimeter[i]);
    }
    return ILQArch {
        width,
        height,
        alg_qubits,
        magic_state_qubits,
        stack_depth,
    };
}

pub fn square_sparse_layout(alg_qubit_count: usize, stack_depth: usize) -> ILQArch {
    let groups = alg_qubit_count.div_ceil(stack_depth) as f64;
    let width = 2 * (groups.sqrt().ceil() as usize) + 3;
    let height = width;
    let mut alg_qubits = Vec::new();
    let interior = |coord| coord > 0 && coord < width - 1;
    for i in 0..width * height {
        let (x, y) = (i % width, i / width);
        if interior(x) && interior(y) && x % 2 == 0 && y % 2 == 0 {
            alg_qubits.push(Location::new(i));
        }
    }
    for i in 0..height {
        for j in 0..width {
            for k in 0..stack_depth {
                if interior(i) && interior(j) && i % 2 == 0 && j % 2 == 0 {
                    alg_qubits.push(Location::new(i * width * stack_depth + j * stack_depth + k));
                }
            }
        }
    }
    let mut perimeter = Vec::new();
    let top_edge = (0..width).map(|i| Location::new(i));
    let mut top_edge = Vec::new();
    for j in 0..width {
        for k in 0..stack_depth {
            top_edge.push(Location::new(0 * width * stack_depth + j * stack_depth + k));
        }
    }
    let right_edge = (1..height).map(|i| Location::new(i * width + width - 1));
    let mut right_edge = Vec::new();
    for i in 1..height {
        for k in 0..stack_depth {
            right_edge.push(Location::new(
                i * width * stack_depth + (width - 1) * stack_depth + k,
            ));
        }
    }
    let bottom_edge = (0..width - 1)
        .rev()
        .map(|i| Location::new(i + width * (height - 1)));
    let mut bottom_edge = Vec::new();
    for j in (0..width - 1).rev() {
        for k in 0..stack_depth {
            bottom_edge.push(Location::new(
                (height - 1) * width * stack_depth + j * stack_depth + k,
            ));
        }
    }
    let mut left_edge = Vec::new();
    for i in (1..height - 1).rev() {
        for k in 0..stack_depth {
            left_edge.push(Location::new(i * width * stack_depth + k));
        }
    }
    perimeter.extend(top_edge);
    perimeter.extend(right_edge);
    perimeter.extend(bottom_edge);
    perimeter.extend(left_edge);
    let mut magic_state_qubits = Vec::new();
    for i in (1..perimeter.len()).step_by(2) {
        magic_state_qubits.push(perimeter[i]);
    }
    return ILQArch {
        width,
        height,
        alg_qubits,
        magic_state_qubits,
        stack_depth,
    };
}

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Clone)]
pub enum ILQGateImplementation {
    Transversal { ctrl: Location, tar: Location },
    LatticeSurgery { path: Vec<Location> },
}
impl GateImplementation for ILQGateImplementation {}
type ILQStep = Step<ILQGateImplementation>;
#[derive(Debug)]
struct IdTransition;
impl Transition<ILQGateImplementation, ILQArch> for IdTransition {
    fn apply(&self, step: &ILQStep) -> ILQStep {
        return ILQStep {
            implemented_gates: HashSet::new(),
            map: step.map.clone(),
        };
    }
    fn repr(&self) -> String {
        return "id".to_string();
    }

    fn cost(&self, _arch: &ILQArch) -> f64 {
        0.0
    }
}

fn ilq_transitions(_step: &ILQStep) -> Vec<IdTransition> {
    return vec![IdTransition];
}

pub fn ilq_step_cost(step: &ILQStep, _arch: &ILQArch) -> f64 {
    if step.implemented_gates.iter().any(|g| {
        matches!(
            g.implementation,
            ILQGateImplementation::LatticeSurgery { .. }
        )
    }) {
        return CODE_DISTANCE as f64;
    } else {
        return 1.0;
    }
}

fn ilq_implement_gate(
    step: &ILQStep,
    arch: &ILQArch,
    gate: &Gate,
) -> Box<dyn Iterator<Item = ILQGateImplementation>> {
    if gate.operation == Operation::CX
        && (step.map[&gate.qubits[0]].get_index() / arch.stack_depth)
            == (step.map[&gate.qubits[1]].get_index() / arch.stack_depth)
    {
        return Box::new(std::iter::once(ILQGateImplementation::Transversal {
            ctrl: step.map[&gate.qubits[0]],
            tar: step.map[&gate.qubits[1]],
        }));
    } else {
        let mut paths: Vec<_> = Vec::new();
        for gate in &step.implemented_gates {
            if let ILQGateImplementation::LatticeSurgery { path } = &gate.implementation {
                paths.extend(path);
            }
        }

        let mapped: Vec<_> = step.map.values().cloned().collect();
        let magic_states = arch.magic_state_qubits.clone();
        let blocked = mapped
            .into_iter()
            .chain(magic_states.into_iter())
            .chain(paths.into_iter())
            .collect();
        let (starts, ends) = match &gate.operation {
            Operation::CX => {
                let (cpos, tpos) = (step.map[&gate.qubits[0]], step.map[&gate.qubits[1]]);
                (
                    vertical_neighbors(cpos, arch.width, arch.height),
                    horizontal_neighbors(tpos, arch.width),
                )
            }
            Operation::T => {
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
            _ => (vec![], vec![]),
        };
        Box::new(
            all_paths(arch, starts, ends, blocked)
                .map(|p| ILQGateImplementation::LatticeSurgery { path: p }),
        )
    }
}

fn mapping_heuristic(_a: &ILQArch, c: &Circuit, m: &QubitMap) -> f64 {
    let mut cost = 0;
    for gate in &c.gates {
        if gate.operation == Operation::CX
            && m[&gate.qubits[0]].get_index() / 4 != m[&gate.qubits[1]].get_index() / 4
        {
            cost += 1;
        }
    }
    return cost as f64;
}

pub fn ilq_solve(c: &Circuit, a: &ILQArch) -> CompilerResult<ILQGateImplementation> {
    return solve(
        c,
        a,
        &ilq_transitions,
        &ilq_implement_gate,
        ilq_step_cost,
        Some(mapping_heuristic),
        true,
    );
}

pub fn ilq_solve_joint_optimize_parallel(
    c: &Circuit,
    a: &ILQArch,
) -> CompilerResult<ILQGateImplementation> {
    return solve_joint_optimize_parallel(
        c,
        a,
        &ilq_transitions,
        &ilq_implement_gate,
        ilq_step_cost,
        Some(mapping_heuristic),
        true,
    );
}
