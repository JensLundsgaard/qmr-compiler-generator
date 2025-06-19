use std::collections::{HashMap, HashSet};

use itertools::Itertools;
use petgraph::{graph::NodeIndex,  Graph};
use serde::Serialize;
use solver::{
    backend::solve,
    structures::{
        Architecture, Circuit, CompilerResult, Gate, GateImplementation, Location, Qubit, Step,
        Transition,
    },
    utils::swap_keys,
};

#[derive(Clone)]
pub struct IonArch {
    trap_size: usize,
    width: usize,
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
        return (0..self.width * self.width * self.trap_size)
            .map(Location::new)
            .collect();
    }

    fn get_trap(&self, loc: Location) -> usize {
        return loc.get_index() / self.trap_size
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
            let loc = pos_to_location.entry((x_pos, y_pos)).or_insert_with(|| {
                *counter += 1;
                Location::new(*counter)
            });
            let v = g.add_node(*loc);
            index_map.insert(*loc, v);
            v
        }
        for i in 0..self.width {
            for j in 0..2 {
                for k in 0..self.trap_size {
                    // add trap locations
                    let (x_pos, y_pos) = (2 * i, 6 * j + k);
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
                        let above = index_map[&pos_to_location[&(2 * i, 6 * j + k2)]];
                        g.add_edge(v, above, ());
                    }
                }
                //add routing nodes, only do this at j=0 because it's shared between rows.
                //                    2*i i+1  
                // -----------------------------
                // j+trap_size   |    v1 \
                //               |    |  v3 
                // j+trap_size+2 |    v2 /
                // -------------------------------
                // i  2*i+1 
                if j == 0 {
                    let (x_pos, y_pos) = (2 * i, 6 * j + self.trap_size);
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
                        index_map[&pos_to_location[&(2 * i, 6 * j + self.trap_size - 1)]],
                        (),
                    );
                    // below
                    let (x_pos, y_pos) = (2 * i, 6 * j + self.trap_size + 2);
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
                    if i < self.width - 1 {
                        let (x_pos, y_pos) = (2 * i + 1, 6 * j + self.trap_size + 1);
                        let v3 = add_location(
                            &mut g,
                            &mut index_map,
                            &mut pos_to_location,
                            &mut counter,
                            x_pos,
                            y_pos,
                        );
                        g.add_edge(v1, v3, ());
                        g.add_edge(v2, v3, ());
                    }
                }
            }
        }
        return (g, index_map);
    }
}

#[derive(Debug)]
pub struct IonTransition {
    edges: Vec<(Location, Location)>,
}

#[derive(Debug)]
pub struct IonTransitionIterator{
    edges: Vec<(Location, Location)>,
    mask: usize,
    max: usize,
}

impl IonTransitionIterator{
    pub fn new(edges : Vec<(Location, Location)>) -> Self{
        let max = 1 << edges.len(); // 2^n combinations
        Self { edges, mask: 0, max } 
    }
}

impl Iterator for IonTransitionIterator{
    type Item = IonTransition;
    fn next(&mut self) -> Option<Self::Item> {
        while self.mask < self.max {
            let mut seen = HashSet::new();
            let mut subset = Vec::new();
            let mut valid = true;

            for i in 0..self.edges.len() {
                if (self.mask >> i) & 1 == 1 {
                    let (a, b) = self.edges[i];
                    if seen.contains(&a) || seen.contains(&b) {
                        valid = false;
                        break;
                    }
                    seen.insert(a);
                    seen.insert(b);
                    subset.push((a, b));
                }
            }

            self.mask += 1;

            if valid {
                return Some(IonTransition { edges: subset });
            }
        }

        None
    }
}
impl Transition<IonGateImplementation, IonArch> for IonTransition {
    fn apply(&self, step: &IonStep) -> IonStep {
        let mut new_step = step.clone();
        for edge in &self.edges{
        new_step.map = swap_keys(&step.map, edge.0, edge.1);
        }
        new_step.implemented_gates = HashSet::new();
        return new_step;
    }

    fn repr(&self) -> String {
        return format!("{:?}", self);
    }

    fn cost(&self, _arch: &IonArch) -> f64 {
        if self.edges.len() == 0{
            0.0
        }
        else{
            1.0
        }
    }
}

fn ion_transitions(arch: &IonArch, step: &IonStep) -> IonTransitionIterator{
    let (graph, _)  = arch.graph();
    let mut edges = vec![];
    for edge in graph.edge_indices(){
        let (u,v) = graph.edge_endpoints(edge).expect("edge should exist");
        let (l1, l2) = (graph[u],graph[v]);
        let exactly_one_mapped = step.map.values().filter(|&& x| x == l1 || x == l2).count() == 1;
        let both_mapped = step.map.values().contains(&l1) &&  step.map.values().contains(&l2);
        let both_trap_positions = arch.get_trap_positions().contains(&l1) && arch.get_trap_positions().contains(&l2);
        if exactly_one_mapped || (both_mapped && both_trap_positions) {
            edges.push((l1, l2));

        }

        
    }
    return IonTransitionIterator::new(edges);


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
