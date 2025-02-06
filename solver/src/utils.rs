use crate::structures::*;

use itertools::max;
use petgraph::graph::NodeIndex;
use petgraph::Direction::Outgoing;
use petgraph::Graph;
use regex::Regex;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{self, BufRead};
use std::iter::from_fn;

pub fn extract_cnots(filename: &str) -> Circuit {
    let file = File::open(filename).unwrap();
    let lines = io::BufReader::new(file).lines();
    let mut gates = Vec::new();
    let mut qubits = HashSet::new();
    let mut id = 0;
    let cx_re = Regex::new(r"cx\s+q\[(\d+)\],\s*q\[(\d+)\];").unwrap();
    for line in lines {
        let line_str = line.unwrap();
        let cx_caps = cx_re.captures(&line_str);
        match cx_caps {
            None => continue,
            Some(c) => {
                let q1 = Qubit::new(c.get(1).unwrap().as_str().parse::<usize>().unwrap());
                let q2 = Qubit::new(c.get(2).unwrap().as_str().parse::<usize>().unwrap());
                qubits.insert(q1);
                qubits.insert(q2);
                let gate = Gate {
                    gate_type: GateType::CX,
                    qubits: vec![q1, q2],
                    id,
                };
                gates.push(gate);
                id += 1;
            }
        }
    }
    return Circuit { gates, qubits };
}

pub fn extract_scmr_gates(filename: &str) -> Circuit {
    let file = File::open(filename).unwrap();
    let lines = io::BufReader::new(file).lines();
    let mut gates = Vec::new();
    let mut qubits = HashSet::new();
    let mut id = 0;
    let cx_re = Regex::new(r"cx\s+q\[(\d+)\],\s*q\[(\d+)\];").unwrap();
    let t_re = Regex::new(r"(t|tdg)\s+q\[(\d+)\];").unwrap();
    for line in lines {
        let line_str = line.unwrap();
        let cx_caps = cx_re.captures(&line_str);
        let t_caps = t_re.captures(&line_str);
        match cx_caps {
            None => match t_caps {
                None => continue,
                Some(c) => {
                    let q = Qubit::new(c.get(2).unwrap().as_str().parse::<usize>().unwrap());
                    qubits.insert(q);
                    let gate = Gate {
                        gate_type: GateType::T,
                        qubits: vec![q],
                        id,
                    };
                    gates.push(gate);
                    id += 1;
                }
            },
            Some(c) => {
                let q1 = Qubit::new(c.get(1).unwrap().as_str().parse::<usize>().unwrap());
                let q2 = Qubit::new(c.get(2).unwrap().as_str().parse::<usize>().unwrap());
                qubits.insert(q1);
                qubits.insert(q2);
                let gate = Gate {
                    gate_type: GateType::CX,
                    qubits: vec![q1, q2],
                    id,
                };
                gates.push(gate);
                id += 1;
            }
        }
    }
    return Circuit { gates, qubits };
}

pub fn path_graph(n: usize) -> Graph<Location, ()> {
    let mut g = Graph::new();
    let mut nodes = Vec::new();
    for i in 0..n {
        nodes.push(g.add_node(Location::new(i)));
    }
    for i in 0..n - 1 {
        g.add_edge(nodes[i], nodes[i + 1], ());
        g.add_edge(nodes[i + 1], nodes[i], ());
    }
    return g;
}

pub fn drop_zeros_and_normalize<T: IntoIterator<Item = (f64, f64)> + Clone>(
    weighted_values: T,
) -> f64 {
    let mut total_weight = 0.0;
    let mut weighted_sum = 0.0;
    for (w, v) in weighted_values.clone() {
        if v != 0.0 {
            total_weight += w;
        }
    }
    for (w, v) in weighted_values.clone() {
        {
            let normalized = w / total_weight;
            weighted_sum += normalized * v;
        }
    }
    return weighted_sum;
}

fn graph_from_edge_vec(edges: Vec<(Location, Location)>) -> Graph<Location, ()> {
    let mut nodes = HashMap::new();
    let mut g = Graph::new();
    for (a, b) in &edges {
        if !nodes.contains_key(a) {
            nodes.insert(a, g.add_node(*a));
        }
        if !nodes.contains_key(b) {
            nodes.insert(b, g.add_node(*b));
        }
        // edges are undirected
        g.update_edge(nodes[a], nodes[b], ());
        g.update_edge(nodes[b], nodes[a], ());
    }
    return g;
}

pub fn graph_from_file(filename: &str) -> Graph<Location, ()> {
    let file = File::open(filename).unwrap();
    let parsed: Value = serde_json::from_reader(file).unwrap();
    let edges = parsed
        .as_array()
        .expect("Expected an array of arrays")
        .iter()
        .map(|inner| {
            let array = inner.as_array().expect("Inner element is not an array");
            if array.len() != 2 {
                panic!("Each edge must have exactly 2 elements");
            }
            let first = array[0]
                .as_u64()
                .expect("Element is not a positive integer") as usize;
            let second = array[1]
                .as_u64()
                .expect("Element is not a positive integer") as usize;
            (Location::new(first), Location::new(second))
        })
        .collect();
    return graph_from_edge_vec(edges);
}

pub fn graph_from_json_entry(entry: Value) -> Graph<Location, ()> {
    let edges = entry
        .as_array()
        .expect("Expected an array of arrays")
        .iter()
        .map(|inner| {
            let array = inner.as_array().expect("Inner element is not an array");
            if array.len() != 2 {
                panic!("Each edge must have exactly 2 elements");
            }
            let first = array[0]
                .as_u64()
                .expect("Element is not a positive integer") as usize;
            let second = array[1]
                .as_u64()
                .expect("Element is not a positive integer") as usize;
            (Location::new(first), Location::new(second))
        })
        .collect();
    return graph_from_edge_vec(edges);
}

pub fn vertical_neighbors(loc: Location, width: usize, height: usize) -> Vec<Location> {
    let mut neighbors = Vec::new();
    if loc.get_index() / width > 0 {
        neighbors.push(Location::new(loc.get_index() - width));
    }
    if loc.get_index() / width < height - 1 {
        neighbors.push(Location::new(loc.get_index() + width));
    }
    return neighbors;
}

pub fn horizontal_neighbors(loc: Location, width: usize) -> Vec<Location> {
    let mut neighbors = Vec::new();
    if loc.get_index() % width > 0 {
        neighbors.push(Location::new(loc.get_index() - 1));
    }
    if loc.get_index() % width < width - 1 {
        neighbors.push(Location::new(loc.get_index() + 1));
    }
    return neighbors;
}

pub fn swap_keys(
    map: &HashMap<Qubit, Location>,
    loc1: Location,
    loc2: Location,
) -> HashMap<Qubit, Location> {
    let mut new_map = map.clone();
    for (qubit, loc) in map {
        if loc == &loc1 {
            new_map.insert(*qubit, loc2);
        } else if loc == &loc2 {
            new_map.insert(*qubit, loc1);
        }
    }
    return new_map;
}

pub fn push_and_return<T: Clone, C: Clone + IntoIterator<Item = T>>(coll: C, item: T) -> Vec<T> {
    let mut new: Vec<T> = coll.into_iter().collect();
    new.push(item);
    return new;
}

pub fn extend_and_return<
    C: Clone + IntoIterator<Item = T>,
    D: Clone + IntoIterator<Item = T>,
    T: Clone,
>(
    left: C,
    right: D,
) -> Vec<T> {
    let mut new: Vec<_> = left.clone().into_iter().collect();
    new.extend(right);
    return new.into_iter().collect();
}
pub fn values<T: Clone, U: Clone>(map: &HashMap<T, U>) -> Vec<U> {
    map.values().cloned().collect()
}

pub fn shortest_path<A: Architecture>(
    arch: &A,
    starts: Vec<Location>,
    ends: Vec<Location>,
    blocked: Vec<Location>,
) -> Option<Vec<Location>> {
    let (mut graph, mut loc_to_node) = arch.graph();
    for loc in blocked.iter() {
        let old_last = graph[graph.node_indices().last().unwrap()];
        graph.remove_node(loc_to_node[loc]);
        loc_to_node.insert(old_last, loc_to_node[loc]);
        loc_to_node.remove(loc);
    }
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
    match best {
        None => None,
        Some((_, path)) => Some(path.into_iter().map(|x| graph[x]).collect()),
    }
}

pub fn identity_application<T: GateImplementation>(step: &Step<T>) -> Step<T> {
    return Step {
        implemented_gates: HashSet::new(),
        map: step.map.clone(),
    };
}
pub fn all_paths<A: Architecture>(
    arch: &A,
    starts: Vec<Location>,
    ends: Vec<Location>,
    blocked: Vec<Location>,
) -> impl Iterator<Item = Vec<Location>> {
    let (mut graph, mut loc_to_node) = arch.graph();
    let max_length = graph.node_count();
    for loc in blocked.iter() {
        let old_last = graph[graph.node_indices().last().unwrap()];
        graph.remove_node(loc_to_node[loc]);
        loc_to_node.insert(old_last, loc_to_node[loc]);
        loc_to_node.remove(loc);
    }

    let unblocked_starts: Vec<_> = starts
        .iter()
        .filter(|x| loc_to_node.contains_key(x))
        .cloned()
        .collect();
    let unblocked_ends: Vec<_> = ends
        .iter()
        .filter(|x| loc_to_node.contains_key(x))
        .cloned()
        .collect();
    let mut start_counter = 0;
    let mut visited = Vec::new();
    let mut stack: Vec<std::vec::IntoIter<NodeIndex>> = Vec::new();
    if !unblocked_starts.is_empty() {
        let start_neighbors: Vec<_> = graph
            .neighbors(loc_to_node[&unblocked_starts[start_counter]])
            .collect();
        stack.push(start_neighbors.into_iter());
        visited.push(unblocked_starts[start_counter]);
    }

    from_fn(move || {
        let mut exhausted = start_counter >= unblocked_starts.len();
        while !exhausted {
            if let Some(children) = stack.last_mut() {
                if let Some(child) = children.next() {
                    let loc = graph[child];
                    if visited.len() < max_length {
                        if ends.contains(&loc) {
                            let path = visited.iter().chain(Some(&loc)).cloned().collect();
                            return Some(path);
                        } else if !visited.contains(&loc) {
                            visited.push(loc);
                            let neighbors: Vec<_> =
                                graph.neighbors_directed(child, Outgoing).collect();
                            let n = neighbors.into_iter();
                            stack.push(n);
                        }
                    } else {
                        if unblocked_ends.contains(&graph[child])
                            || children.any(|x| unblocked_ends.contains(&graph[x]))
                        {
                            let path = visited.iter().chain(Some(&loc)).cloned().collect();
                            return Some(path);
                        }
                        stack.pop();
                        visited.pop();
                    }
                } else {
                    stack.pop();
                    visited.pop();
                }
            } else {
                start_counter += 1;
                if start_counter < unblocked_starts.len() {
                    visited = vec![unblocked_starts[start_counter]];
                    let start_neighbors: Vec<_> = graph
                        .neighbors(loc_to_node[&unblocked_starts[start_counter]])
                        .collect();
                    stack.push(start_neighbors.into_iter());
                } else {
                    exhausted = true;
                }
            }
        }
        None
    })
}

pub fn build_criticality_table(c: &Circuit) -> HashMap<usize, usize> {
    let mut qubit_table: HashMap<usize, usize> = HashMap::new();
    let mut gate_table: HashMap<usize, usize> = HashMap::new();
    for gate in &c.gates {
        let d = max(c.qubits.iter().map(|x| qubit_table.get(&x.get_index())))
            .flatten()
            .copied()
            .unwrap_or_default();
        gate_table.insert(gate.id, d + 1);
        for q in &c.qubits {
            qubit_table.insert(q.get_index(), d + 1);
        }
    }
    gate_table
}
