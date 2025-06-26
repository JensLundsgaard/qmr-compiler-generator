use std::collections::{HashMap, HashSet};

use itertools::{any, Itertools};
use petgraph::{graph::NodeIndex, Graph};
use serde::Serialize;
use solver::{
    backend::{solve, solve_joint_optimize_parallel},
    structures::{
        Architecture, Circuit, CompilerResult, Gate, GateImplementation, Location, Qubit, Step,
        Transition,
    },
    utils::swap_keys,
};

const MERGE_COST: f64 = 80e-6;
const SPLIT_COST: f64 = 80e-6;
const SEGMENT_COST: f64 = 5e-6;
const INNER_SWAP_COST: f64 = 42e-6;
const Y_COST: f64 = 100e-6;
const X_COST: f64 = 120e-6;

#[derive(Clone)]
pub struct IonArch {
    pub trap_size: usize,
    pub width: usize,
}

impl Architecture for IonArch {
    fn locations(&self) -> Vec<Location> {
        return self.get_trap_positions();
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

impl IonArch {
    fn get_trap_positions(&self) -> Vec<Location> {
        return (0..self.width * 2 * self.trap_size)
            .map(Location::new)
            .collect();
    }

    fn get_trap(&self, loc: Location) -> usize {
        return loc.get_index() / self.trap_size;
    }

    fn get_outer_trap_positions(&self) -> Vec<Location> {
        let mut locs = vec![];
        for loc in self.get_trap_positions() {
            let top_row_outer = (loc.get_index() / self.trap_size) % 2 == 0
                && ((loc.get_index() % self.trap_size) == self.trap_size - 1);
            let bottom_row_outer = (loc.get_index() / self.trap_size) % 2 == 1
                && ((loc.get_index() % self.trap_size) == 0);
            if top_row_outer || bottom_row_outer {
                locs.push(loc);
            }
        }
        return locs;
    }
    fn get_graph(&self) -> (Graph<Location, ()>, HashMap<Location, NodeIndex>) {
        let mut g = Graph::new();
        let mut index_map = HashMap::new();
        let mut pos_to_location: HashMap<(usize, usize), Location> = HashMap::new();
        let mut counter = 0;
        fn add_location(
            g: &mut Graph<Location, ()>,
            index_map: &mut HashMap<Location, NodeIndex>,
            pos_to_location: &mut HashMap<(usize, usize), Location>,
            counter: &mut usize,
            x_pos: usize,
            y_pos: usize,
        ) -> NodeIndex {
            if pos_to_location.contains_key(&(x_pos, y_pos)) {
                panic!("already seen this position")
            } else {
                let loc = Location::new(*counter);
                pos_to_location.insert((x_pos, y_pos), loc);
                *counter += 1;
                let v = g.add_node(loc);
                index_map.insert(loc, v);
                v
            }
        }
        for i in 0..self.width {
            for j in 0..2 {
                for k in 0..self.trap_size {
                    // add trap locations
                    let (x_pos, y_pos) = (2 * i, (self.trap_size + 3) * j + k);
                    let v = add_location(
                        &mut g,
                        &mut index_map,
                        &mut pos_to_location,
                        &mut counter,
                        x_pos,
                        y_pos,
                    );
                    // add all-to-all connectivity within traps
                    for k2 in 0..k {
                        let above =
                            index_map[&pos_to_location[&(2 * i, (self.trap_size + 3) * j + k2)]];
                        g.add_edge(v, above, ());
                        g.add_edge(above, v, ());
                    }
                }
            }
        }
        for i in 0..self.width {
            for j in 0..2 {
                //add routing nodes, only do this at j=0 because it's shared between rows.
                //                    2*i i+1
                // -----------------------------
                // j+trap_size   |    v1 \
                //               |    |  v3
                // j+trap_size+2 |    v2 /
                // -------------------------------
                // i  2*i+1
                if j == 0 {
                    let (x_pos, y_pos) = (2 * i, self.trap_size);
                    let v1 = add_location(
                        &mut g,
                        &mut index_map,
                        &mut pos_to_location,
                        &mut counter,
                        x_pos,
                        y_pos,
                    );
                    // trap to routing channel
                    g.add_edge(
                        v1,
                        index_map[&pos_to_location[&(2 * i, self.trap_size - 1)]],
                        (),
                    );
                    g.add_edge(
                        index_map[&pos_to_location[&(2 * i, self.trap_size - 1)]],
                        v1,
                        (),
                    );
                    // below
                    let (x_pos, y_pos) = (2 * i, self.trap_size + 2);
                    let v2 = add_location(
                        &mut g,
                        &mut index_map,
                        &mut pos_to_location,
                        &mut counter,
                        x_pos,
                        y_pos,
                    );
                    // junction
                    g.add_edge(v1, v2, ());
                    g.add_edge(v2, v1, ());
                    if i < self.width - 1 {
                        let (x_pos, y_pos) = (2 * i + 1, self.trap_size + 1);
                        let v3 = add_location(
                            &mut g,
                            &mut index_map,
                            &mut pos_to_location,
                            &mut counter,
                            x_pos,
                            y_pos,
                        );
                        g.add_edge(v1, v3, ());
                        g.add_edge(v3, v1, ());
                        g.add_edge(v2, v3, ());
                        g.add_edge(v3, v2, ());
                    }
                    if i > 0 {
                        let (x_pos, y_pos) = (2 * i - 1, self.trap_size + 1);
                        let v4 = index_map[&pos_to_location[&(x_pos, y_pos)]];
                        g.add_edge(v1, v4, ());
                        g.add_edge(v4, v1, ());
                        g.add_edge(v2, v4, ());
                        g.add_edge(v4, v2, ());
                        if i < self.width - 1 {
                            let (x_pos, y_pos) = (2 * i + 1, self.trap_size + 1);
                            let v3 = index_map[&pos_to_location[&(x_pos, y_pos)]];
                            g.add_edge(v3, v4, ());
                            g.add_edge(v4, v3, ());
                        }
                    }
                } else {
                    let (outermost_trap_pos_x, outermost_trap_pos_y) = (2 * i, self.trap_size + 3);
                    let v =
                        index_map[&pos_to_location[&(outermost_trap_pos_x, outermost_trap_pos_y)]];
                    let above = index_map[&pos_to_location[&(2 * i, self.trap_size + 2)]];
                    g.add_edge(v, above, ());
                    g.add_edge(above, v, ());
                }
            }
        }
        return (g, index_map);
    }
}

#[derive(Debug)]
pub struct IonTransition {
    pairs: Vec<(Location, Location)>,
}

#[derive(Debug)]
pub struct IonTransitionIterator {
    pairs: Vec<(Location, Location)>,
    mask: usize,
    max: usize,
    trap_size: usize,
}

impl IonTransitionIterator {
    pub fn new(pairs: Vec<(Location, Location)>, trap_size: usize) -> Self {
        let max = 1 << pairs.len(); // 2^n combinations
        Self {
            pairs,
            mask: 0,
            max,
            trap_size,
        }
    }
}

fn consistent(seen_set: &HashSet<(usize, usize)>, new: (usize, usize)) -> bool {
    for pair in seen_set {
        let safe = pair.1 < new.0 || pair.0 < new.1;
        if !safe {
            return false;
        }
    }
    return true;
}

impl Iterator for IonTransitionIterator {
    type Item = IonTransition;
    fn next(&mut self) -> Option<Self::Item> {
        while self.mask < self.max {
            let mut seen_pairs = HashSet::new();
            let mut seen_locs = HashSet::new();
            let mut subset = Vec::new();
            let mut valid = true;

            for i in 0..self.pairs.len() {
                if (self.mask >> i) & 1 == 1 {
                    let (a, b) = self.pairs[i];
                    let (col_a, col_b) = (
                        a.get_index() / (2 * self.trap_size),
                        b.get_index() / (2 * self.trap_size),
                    );
                    let same_trap =
                        a.get_index() / self.trap_size == b.get_index() / self.trap_size;
                    let (min_col, max_col) = if col_a < col_b {
                        (col_a, col_b)
                    } else {
                        (col_b, col_a)
                    };
                    let addable = (consistent(&seen_pairs, (min_col, max_col)) || same_trap)
                        && !seen_locs.contains(&a)
                        && !seen_locs.contains(&b);
                    if !addable {
                        valid = false;
                        break;
                    }
                    seen_pairs.insert((min_col, max_col));
                    seen_locs.insert(a);
                    seen_locs.insert(b);
                    subset.push((a, b));
                }
            }

            self.mask += 1;

            if valid {
                return Some(IonTransition { pairs: subset });
            }
        }

        None
    }
}

fn get_pair_cost(pair: (Location, Location), arch: &IonArch) -> f64 {
    let mut cost = 0.0;
    // all pairs have these at the end points
    cost += SPLIT_COST + SEGMENT_COST + SEGMENT_COST + MERGE_COST;
    let (col_a, col_b) = (
        pair.0.get_index() / (2 * arch.trap_size),
        pair.0.get_index() / (2 * arch.trap_size),
    );
    // counting junctions
    let junction_count = usize::abs_diff(col_a, col_b);
    if junction_count > 0 {
        let mut y_count = 0;
        if col_a == 0 || col_a == arch.width - 1 {
            y_count += 1;
        }
        if col_b == 0 || col_b == arch.width - 1 {
            y_count += 1;
        }
        let x_count = junction_count - y_count;
        cost += y_count as f64 * (Y_COST + SEGMENT_COST);
        cost += x_count as f64 * (Y_COST + SEGMENT_COST);
    }
    if !arch.get_outer_trap_positions().contains(&pair.0) {
        cost += INNER_SWAP_COST;
    }
    if !arch.get_outer_trap_positions().contains(&pair.1) {
        cost += INNER_SWAP_COST;
    }
    return cost;
}

impl Transition<IonGateImplementation, IonArch> for IonTransition {
    fn apply(&self, step: &IonStep) -> IonStep {
        let mut new_step = step.clone();
        for pair in &self.pairs {
            new_step.map = swap_keys(&new_step.map, pair.0, pair.1);
        }
        new_step.implemented_gates = HashSet::new();
        return new_step;
    }

    fn repr(&self) -> String {
        return format!("{:?}", self);
    }

    fn cost(&self, arch: &IonArch) -> f64 {
        if self.pairs.len() == 0 {
            0.0
        } else {
            self.pairs
                .iter()
                .map(|pair| get_pair_cost(*pair, arch))
                .max_by(|a, b| a.total_cmp(b))
                .unwrap_or(0.0)
        }
    }
}

fn ion_transitions(arch: &IonArch, step: &IonStep) -> IonTransitionIterator {
    let (graph, _) = arch.graph();
    let mut edges = vec![];

    let outers = arch.get_trap_positions();
    let map_positions  : Vec<_>= step.map.values().collect();
    for outer1 in &outers {
        for outer2 in &outers {
            if outer1 != outer2 && (map_positions.contains(&outer1) || map_positions.contains(&outer2)) {
                edges.push((*outer1, *outer2));
            }
        }
    }
    println!("edges: {:?}", edges);
    return IonTransitionIterator::new(edges, arch.trap_size);
}

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Clone)]
pub struct IonGateImplementation {
    u: Location,
    v: Location,
}
impl GateImplementation for IonGateImplementation {}
type IonStep = Step<IonGateImplementation>;
fn ion_implement_gate(
    step: &IonStep,
    arch: &IonArch,
    gate: &Gate,
) -> Option<IonGateImplementation> {
    let (cpos, tpos) = (step.map.get(&gate.qubits[0]), step.map.get(&gate.qubits[1]));
    match (cpos, tpos) {
        (Some(cpos), Some(tpos)) if arch.get_trap(*cpos) == arch.get_trap(*tpos) => {
            Some(IonGateImplementation { u: *cpos, v: *tpos })
        }
        _ => None,
    }
}

fn mapping_heuristic(arch: &IonArch, c: &Circuit, map: &HashMap<Qubit, Location>) -> f64 {
    let (graph, index_map) = arch.get_graph();
    // println!("map : {:?}", map);
    // println!("locations : {:?}", arch.get_trap_positions());
    let mut cost = 0;
    for gate in &c.gates {
        let (cpos, tpos) = (map.get(&gate.qubits[0]), map.get(&gate.qubits[1]));
        let (cind, tind) = (index_map[cpos.unwrap()], index_map[tpos.unwrap()]);
        let sp_res = petgraph::algo::astar(&graph, cind, |n| n == tind, |_| 1, |_| 0);

        match sp_res {
            Some((c, _)) => {
                cost += c;
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

pub fn ion_solve(c: &Circuit, a: &IonArch) -> CompilerResult<IonGateImplementation> {
    return solve(
        c,
        a,
        &|s| ion_transitions(a, s),
        &ion_implement_gate,
        |_s, _a| 0.0,
        Some(mapping_heuristic),
        false,
    );
}
pub fn ion_solve_joint_optimize_parallel(
    c: &Circuit,
    a: &IonArch,
) -> CompilerResult<IonGateImplementation> {
    return solve_joint_optimize_parallel(
        c,
        a,
        &|s| ion_transitions(a, s),
        &ion_implement_gate,
        |_s, _a| 0.0,
        Some(mapping_heuristic),
        false,
    );
}
