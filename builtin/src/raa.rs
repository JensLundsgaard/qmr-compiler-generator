use serde::Serialize;

use solver::{
    backend::{sabre_solve, solve},
    structures::*,
};
use std::collections::{HashMap, HashSet};

const ACCELERATION_CONST: f64 = 2750.0;
const ATOM_TRANSFER_FIDELITY: f64 = 0.999;
const ATOM_TRANSFER_TIME: f64 = 15.0e-6;

const EXCITEMENT_FIDELITY: f64 = 0.9975;
const RYDBERG_RADIUS: f64 = 6.0e-6;
const TWO_QUBIT_GATE_FIDELITY: f64 = 0.995;
const T2: f64 = 1.5;

#[derive(Clone)]
pub struct RaaArchitecture {
    pub width: usize,
    pub height: usize,
}

impl Architecture for RaaArchitecture {
    fn locations(&self) -> Vec<Location> {
        let mut locations = Vec::new();
        for i in 0..self.width {
            for j in 0..self.height {
                let loc = Location::new(i * self.height + j);
                locations.push(loc);
            }
        }
        return locations;
    }

    fn graph(
        &self,
    ) -> (
        petgraph::Graph<Location, ()>,
        HashMap<Location, petgraph::prelude::NodeIndex>,
    ) {
        let mut g = petgraph::Graph::new();
        let mut index_map = HashMap::new();
        for i in 0..self.width {
            for j in 0..self.height {
                let loc = Location::new(i * self.height + j);
                let v = g.add_node(loc);
                index_map.insert(loc, v);
            }
        }
        for i in 0..self.width {
            for j in 0..self.height {
                // edge to above
                if i > 0 {
                    let v1 = index_map[&Location::new(i * self.height + j)];
                    let v2 = index_map[&Location::new((i - 1) * self.height + j)];
                    g.add_edge(v1, v2, ());
                    g.add_edge(v2, v1, ());
                }
                // edge to below
                if i < self.width - 1 {
                    let v1 = index_map[&Location::new(i * self.height + j)];
                    let v2 = index_map[&Location::new((i + 1) * self.height + j)];
                    g.add_edge(v1, v2, ());
                    g.add_edge(v2, v1, ());
                }
                // edge to left
                if j > 0 {
                    let v1 = index_map[&Location::new(i * self.height + j)];
                    let v2 = index_map[&Location::new(i * self.height + j - 1)];
                    g.add_edge(v1, v2, ());
                    g.add_edge(v2, v1, ());
                }
                // edge to right
                if j < self.height - 1 {
                    let v1 = index_map[&Location::new(i * self.height + j)];
                    let v2 = index_map[&Location::new(i * self.height + j + 1)];
                    g.add_edge(v1, v2, ());
                    g.add_edge(v2, v1, ());
                }
            }
        }
        return (g, index_map);
    }
}
struct IdTransition;
#[derive(Clone, Debug, Serialize, PartialEq, Eq, Hash)]
pub struct RaaGateImplementation {
    src: Location,
    dst: Location,
}

impl GateImplementation for RaaGateImplementation {}

type RaaStep = Step<RaaGateImplementation>;

impl Transition<RaaGateImplementation, RaaArchitecture> for IdTransition {
    fn apply(&self, step: &RaaStep) -> RaaStep {
        return RaaStep {
            implemented_gates: HashSet::new(),
            map: step.map.clone(),
        };
    }
    fn repr(&self) -> String {
        return "id".to_string();
    }

    fn cost(&self, _arch: &RaaArchitecture) -> f64 {
        0.0
    }
}

fn raa_transitions() -> Vec<IdTransition> {
    return vec![IdTransition];
}
#[derive(Clone, Debug)]
struct RaaMove {
    qubit: Qubit,
    dst: Location,
    cost: f64,
}

impl Transition<RaaGateImplementation, RaaArchitecture> for RaaMove {
    fn apply(&self, step: &RaaStep) -> RaaStep {
        let mut new_step = step.clone();
        new_step.map.insert(self.qubit, self.dst);
        new_step.implemented_gates = HashSet::new();
        return new_step;
    }
    fn repr(&self) -> String {
        format!("RELOCATE {:?} {:?}", self.qubit, self.dst)
    }

    fn cost(&self, _arch: &RaaArchitecture) -> f64 {
        self.cost
    }
}

fn raa_transitions_dyn_map(step: &RaaStep, arch: &RaaArchitecture) -> Vec<RaaMove> {
    let mut moves = Vec::new();
    let impls = step
        .implemented_gates
        .iter()
        .map(|gi| gi.implementation.clone());
    for raa_move in impls {
        let aod_qubit = step
            .map
            .iter()
            .find(|(_q, l)| *l == &raa_move.src)
            .unwrap()
            .0;
        let slm_qubit = step
            .map
            .iter()
            .find(|(_q, l)| *l == &raa_move.dst)
            .unwrap()
            .0;
        for dst in arch.locations() {
            if !(step.map.values().any(|v| v == &dst && v != &raa_move.src)) {
                let src_coords = (
                    step.map.get(slm_qubit).unwrap().get_index() / arch.height,
                    step.map.get(slm_qubit).unwrap().get_index() % arch.height,
                );
                let dst_coords = (dst.get_index() / arch.height, dst.get_index() % arch.height);
                let dist = f64::sqrt(
                    (src_coords.0 as f64 - dst_coords.0 as f64).powi(2)
                        + (src_coords.1 as f64 - dst_coords.1 as f64).powi(2),
                );
                let move_time = f64::sqrt(2.5 * RYDBERG_RADIUS * dist / ACCELERATION_CONST);
                let cost = -f64::ln(1.0 - move_time / T2);
                moves.push(RaaMove {
                    qubit: *aod_qubit,
                    dst,
                    cost,
                });
            }
        }
    }
    return moves;
}

fn consistent(
    shuttle: ((usize, usize), (usize, usize)),
    row_displacements: &HashMap<usize, usize>,
    col_displacements: &HashMap<usize, usize>,
) -> bool {
    let (src, dst) = shuttle;
    let (src_col, src_row) = src;
    let (dst_col, dst_row) = dst;
    let inverse_col_displacements: HashMap<usize, usize> =
        col_displacements.iter().map(|(k, v)| (*v, *k)).collect();
    let inverse_row_displacements: HashMap<usize, usize> =
        row_displacements.iter().map(|(k, v)| (*v, *k)).collect();
    if col_displacements.contains_key(&src_col) && col_displacements[&src_col] != dst_col {
        return false;
    } else if inverse_col_displacements.contains_key(&dst_col)
        && src_col != inverse_col_displacements[&dst_col]
    {
        return false;
    } else if col_displacements
        .iter()
        .any(|(k, v)| k > &src_col && v <= &dst_col)
    {
        return false;
    } else if row_displacements.contains_key(&src_row) && row_displacements[&src_row] != dst_row {
        return false;
    } else if inverse_row_displacements.contains_key(&dst_row)
        && src_row != inverse_row_displacements[&dst_row]
    {
        return false;
    } else if row_displacements
        .iter()
        .any(|(k, v)| k > &src_row && v <= &dst_row)
    {
        return false;
    } else {
        return true;
    }
}

fn raa_step_valid(step: &RaaStep, arch: &RaaArchitecture) -> bool {
    let mut row_displacements: HashMap<usize, usize> = HashMap::new();
    let mut col_displacements: HashMap<usize, usize> = HashMap::new();
    for gate in &step.gates() {
        let ctrl_coords = (
            step.map[&gate.qubits[0]].get_index() / arch.height,
            step.map[&gate.qubits[0]].get_index() % arch.height,
        );
        let tar_coords = (
            step.map[&gate.qubits[1]].get_index() / arch.height,
            step.map[&gate.qubits[1]].get_index() % arch.height,
        );
        let move_ctrl_to_tar = (ctrl_coords, tar_coords);
        let move_tar_to_ctrl = (tar_coords, ctrl_coords);
        if consistent(move_ctrl_to_tar, &row_displacements, &col_displacements) {
            row_displacements.insert(ctrl_coords.1, tar_coords.1);
            col_displacements.insert(ctrl_coords.0, tar_coords.0);
        } else if consistent(move_tar_to_ctrl, &row_displacements, &col_displacements) {
            row_displacements.insert(tar_coords.1, ctrl_coords.1);
            col_displacements.insert(tar_coords.0, ctrl_coords.0);
        } else {
            return false;
        }
    }
    return true;
}

fn raa_implement_gate(
    step: &RaaStep,
    arch: &RaaArchitecture,
    gate: &Gate,
) -> Vec<RaaGateImplementation> {
    let ctrl_coords = (
        step.map[&gate.qubits[0]].get_index() / arch.height,
        step.map[&gate.qubits[0]].get_index() % arch.height,
    );
    let tar_coords = (
        step.map[&gate.qubits[1]].get_index() / arch.height,
        step.map[&gate.qubits[1]].get_index() % arch.height,
    );
    let mut row_displacements: HashMap<usize, usize> = HashMap::new();
    let mut col_displacements: HashMap<usize, usize> = HashMap::new();
    let existing_moves = step.implemented_gates.iter().map(|g| {
        (
            (
                g.implementation.src.get_index() / arch.height,
                g.implementation.src.get_index() % arch.height,
            ),
            (
                g.implementation.dst.get_index() / arch.height,
                g.implementation.dst.get_index() % arch.height,
            ),
        )
    });
    for ((src_row, src_col), (dst_row, dst_col)) in existing_moves {
        row_displacements.insert(src_row, dst_row);
        col_displacements.insert(src_col, dst_col);
    }

    let move_ctrl_to_tar = (ctrl_coords, tar_coords);
    let move_tar_to_ctrl = (tar_coords, ctrl_coords);
    let mut v = Vec::new();
    if consistent(move_ctrl_to_tar, &row_displacements, &col_displacements) {
        v.push(RaaGateImplementation {
            src: step.map[&gate.qubits[0]],
            dst: step.map[&gate.qubits[1]],
        });
    }
    if consistent(move_tar_to_ctrl, &row_displacements, &col_displacements) {
        v.push(RaaGateImplementation {
            src: step.map[&gate.qubits[1]],
            dst: step.map[&gate.qubits[0]],
        });
    }
    return v;
}

fn raa_step_cost(step: &RaaStep, arch: &RaaArchitecture) -> f64 {
    let mut cost = 0.0;
    let mut max_dist = 0.0;
    for gate in &step.gates() {
        let ctrl_coords = (
            step.map[&gate.qubits[0]].get_index() / arch.height,
            step.map[&gate.qubits[0]].get_index() % arch.height,
        );
        let tar_coords = (
            step.map[&gate.qubits[1]].get_index() / arch.height,
            step.map[&gate.qubits[0]].get_index() % arch.height,
        );
        let dist = f64::sqrt(
            (ctrl_coords.0 as f64 - tar_coords.0 as f64).powi(2)
                + (ctrl_coords.1 as f64 - tar_coords.1 as f64).powi(2),
        );
        if dist > max_dist {
            max_dist = dist;
        }
    }
    let move_time = f64::sqrt(2.5 * RYDBERG_RADIUS * max_dist / ACCELERATION_CONST);
    let gates = step.gates();
    let active_qubits: HashSet<&Qubit> = gates.iter().flat_map(|g| &g.qubits).collect();
    let active_qubit_count = active_qubits.len();
    let inactive_qubit_count = step.map.len() - active_qubit_count;
    // two qubit gate fidelity term
    cost += -f64::ln(TWO_QUBIT_GATE_FIDELITY) * (gates.len() as f64);
    // atom transfer
    cost += -f64::ln(ATOM_TRANSFER_FIDELITY) * (active_qubit_count as f64);
    // decoherence for active qubits
    for _ in 1..active_qubit_count {
        cost += -f64::ln(1.0 - (move_time / T2));
    }
    // decoherence for inactive qubits + excited but not gate
    for _ in 1..inactive_qubit_count {
        cost += -f64::ln(1.0 - (move_time + 4.0 * ATOM_TRANSFER_TIME) / T2);
        cost += -f64::ln(EXCITEMENT_FIDELITY);
    }
    return cost;
}

pub fn raa_solve(c: &Circuit, arch: &RaaArchitecture) -> CompilerResult<RaaGateImplementation> {
    solve(
        c,
        arch,
        &|s| raa_transitions_dyn_map(s, arch),
        raa_implement_gate,
        raa_step_cost,
        None,
        true,
    )
}

pub fn raa_solve_sabre(
    c: &Circuit,
    arch: &RaaArchitecture,
) -> CompilerResult<RaaGateImplementation> {
    sabre_solve(
        c,
        arch,
        &|s| raa_transitions_dyn_map(s, arch),
        &raa_implement_gate,
        raa_step_cost,
        None,
        true,
    )
}
