use petgraph::graph::NodeIndex;
use rand::seq::IndexedRandom;

use crate::structures::*;
use crate::utils::*;
use std::collections::HashSet;
use std::thread;
use std::time::Duration;
use std::{collections::HashMap, fmt::Debug};
const ALPHA: f64 = 1.0;
const BETA: f64 = 1.0;
const GAMMA: f64 = 1.0;
const DELTA: f64 = 1.0;
const INITIAL_TEMP: f64 = 10.0;
const TERM_TEMP: f64 = 0.00001;
const COOL_RATE: f64 = 0.99;
const SABRE_ITERATIONS: usize = 3;
const ISOM_SEARCH_TIMEOUT : Duration = Duration::from_secs(300);

fn random_map<T: Architecture>(c: &Circuit, arch: &T) -> QubitMap {
    let mut map = HashMap::new();
    let mut rng = &mut rand::rng();
    let locations = arch.locations();
    let v = locations.choose_multiple(&mut rng, c.qubits.len());
    for (q, l) in c.qubits.iter().zip(v) {
        map.insert(*q, *l);
    }
    return map;
}

fn isomorphism_map<T: Architecture>(c: &Circuit, arch: &T) -> Option<QubitMap> {
    let interact_graph = build_interaction_graph(c);
    let (graph, loc_to_node) = arch.graph();
    let isom = vf2::subgraph_isomorphisms(&interact_graph, &graph).first();
    isom.map(|v| {
        v.iter()
            .enumerate()
            .map(|(q, i)| (interact_graph[NodeIndex::new(q)], graph[NodeIndex::new(*i)]))
            .collect()
    })
}

fn isomorphism_map_with_timeout<T: Architecture + Send + Sync + Clone + 'static>(
    c: &Circuit,
    arch: &T,
    timeout: Duration,
) -> Option<QubitMap> {
    let (tx, rx) = std::sync::mpsc::channel();
    let c_clone = c.clone();
    let arch_clone = arch.clone();
    thread::spawn(move || {
        let result = isomorphism_map(&c_clone, &arch_clone);
        let _ = tx.send(result);
    });

    match rx.recv_timeout(timeout) {
        Ok(res) => res,
        Err(_) => None,
    }
}



fn random_neighbor<T: Architecture>(map: &QubitMap, arch: &T) -> QubitMap {
    let mut moves: Vec<Box<dyn Fn(&QubitMap) -> QubitMap>> = Vec::new();
    for q1 in map.keys() {
        for q2 in map.keys() {
            if q1 == q2 {
                continue;
            }
            let swap_keys = |m: &QubitMap| {
                let mut new_map = m.clone();
                let loc1 = m.get(q1).unwrap();
                let loc2 = m.get(q2).unwrap();
                new_map.insert(*q1, *loc2);
                new_map.insert(*q2, *loc1);
                return new_map;
            };
            moves.push(Box::new(swap_keys));
        }
    }
    for q in map.keys() {
        for l in arch.locations() {
            if !map.values().any(|x| *x == l) {
                let l = l.clone();
                let into_open = move |m: &QubitMap| {
                    let mut new_map = m.clone();
                    new_map.insert(*q, l);
                    return new_map;
                };
                moves.push(Box::new(into_open));
            }
        }
    }
    let rng = &mut rand::rng();
    let chosen_move = moves.choose(rng).unwrap();
    return chosen_move(&map);
}

fn sim_anneal_mapping_search<T: Architecture>(
    start: QubitMap,
    arch: &T,
    initial_temp: f64,
    term_temp: f64,
    cool_rate: f64,
    heuristic: impl Fn(&QubitMap) -> f64,
) -> QubitMap {
    return simulated_anneal(
        start,
        initial_temp,
        term_temp,
        cool_rate,
        |m| random_neighbor(m, arch),
        heuristic,
    );
}

fn route<
    A: Architecture,
    R: Transition<G, A> + Debug,
    G: GateImplementation + Debug,
    I: IntoIterator<Item = G>,
>(
    c: &Circuit,
    arch: &A,
    map: QubitMap,
    transitions: &impl Fn(&Step<G>) -> Vec<R>,
    implement_gate: impl Fn(&Step<G>, &A, &Gate) -> I,
    step_cost: fn(&Step<G>, &A) -> f64,
    map_eval: &impl Fn(&Circuit, &QubitMap) -> f64,
    explore_routing_orders: bool,
    crit_table: &HashMap<usize, usize>,
) -> CompilerResult<G> {
    let mut steps = Vec::new();
    let mut trans_taken = Vec::new();
    let mut step_0 = Step {
        map,
        implemented_gates: HashSet::new(),
    };
    let mut current_circ = c.clone();
    let mut cost = step_cost(&step_0, arch);
    let executable = &c.get_front_layer();
    if explore_routing_orders {
        step_0.max_step_all_orders(executable, arch, &implement_gate, crit_table);
    } else {
        step_0.max_step(executable, arch, &implement_gate);
    }
    current_circ.remove_gates(&(step_0.gates()));
    steps.push(step_0);
    while current_circ.gates.len() > 0 {
        let best = find_best_next_step(
            &current_circ,
            arch,
            &transitions,
            &implement_gate,
            steps.last().unwrap(),
            step_cost,
            &map_eval,
            explore_routing_orders,
            &crit_table,
        );
        match best {
            Some((s, trans, _b)) => {
                current_circ.remove_gates(&s.gates());
                cost += step_cost(&s, arch);
                steps.push(s);
                trans_taken.push(trans.repr());
                cost += trans.cost(arch);
            }
            None => {
                panic!("No valid next step found");
            }
        }
    }
    return CompilerResult {
        steps,
        transitions: trans_taken,
        cost,
    };
}

fn find_best_next_step<
    A: Architecture,
    R: Transition<G, A>,
    G: GateImplementation,
    I: IntoIterator<Item = G>,
>(
    c: &Circuit,
    arch: &A,
    transitions: &impl Fn(&Step<G>) -> Vec<R>,
    implement_gate: impl Fn(&Step<G>, &A, &Gate) -> I,
    last_step: &Step<G>,
    step_cost: fn(&Step<G>, &A) -> f64,
    map_eval: impl Fn(&Circuit, &QubitMap) -> f64,
    explore_routing_orders: bool,
    crit_table: &HashMap<usize, usize>,
) -> Option<(Step<G>, R, f64)> {
    let mut best: Option<(Step<G>, R, f64)> = None;
    for trans in transitions(last_step) {
        let mut next_step = trans.apply(last_step);
        let executable = c.get_front_layer();
        if explore_routing_orders {
            next_step.max_step_all_orders(&executable, arch, &implement_gate, crit_table);
        } else {
            next_step.max_step(&executable, arch, &implement_gate);
        }
        let s_cost = step_cost(&next_step, arch);
        let t_cost = trans.cost(arch);
        let m_cost = map_eval(&circuit_from_gates(executable), &next_step.map);
        let total_criticality: usize = next_step
            .gates()
            .into_iter()
            .map(|x| crit_table[&x.id])
            .sum();
        let weighted_vals = std::iter::zip(
            vec![ALPHA, BETA, GAMMA, DELTA],
            vec![s_cost, t_cost, m_cost, -(total_criticality as f64)],
        );
        let cost = drop_zeros_and_normalize(weighted_vals);
        match best {
            Some((ref _s, ref _prev_trans, b)) => {
                if cost < b {
                    best = Some((next_step, trans, cost));
                }
            }
            None => {
                best = Some((next_step, trans, cost));
            }
        }
    }
    return best;
}

pub fn solve<
    A: Architecture + Send + Sync + Clone + 'static,
    R: Transition<G, A> + Debug,
    G: GateImplementation + Debug,
    I: IntoIterator<Item = G>,
>(
    c: &Circuit,
    arch: &A,
    transitions: &impl Fn(&Step<G>) -> Vec<R>,
    implement_gate: fn(&Step<G>, &A, &Gate) -> I,
    step_cost: fn(&Step<G>, &A) -> f64,
    mapping_heuristic: Option<fn(&A, &Circuit, &QubitMap) -> f64>,
    explore_routing_orders: bool,
) -> CompilerResult<G> {
    let crit_table = &build_criticality_table(c);
    match mapping_heuristic {
        Some(heuristic) => {
            let map_h = |m: &QubitMap| heuristic(arch, c, m);
            let route_h = |c: &Circuit, m: &QubitMap| heuristic(arch, c, m);
            let isom_map = isomorphism_map_with_timeout(c, arch, ISOM_SEARCH_TIMEOUT);
            let isom_cost = isom_map.clone().map(|x| map_h(&x));
            let sa_map = match isom_cost {
                Some(c) if c == 0.0 => None,
                _ => Some(sim_anneal_mapping_search(
                    random_map(c, arch),
                    arch,
                    INITIAL_TEMP,
                    TERM_TEMP,
                    COOL_RATE,
                    map_h,
                )),
            };
            let sa_cost = sa_map.clone().map(|x| map_h(&x));
            let map = match (isom_cost, sa_cost) {
                (Some(i_c), Some(s_c)) if i_c < s_c => isom_map.unwrap(),
                _ => sa_map.unwrap(),
            };
            return route(
                c,
                arch,
                map,
                transitions,
                implement_gate,
                step_cost,
                &route_h,
                explore_routing_orders,
                crit_table,
            );
        }
        None => {
            let map = random_map(c, arch);
            return route(
                c,
                arch,
                map,
                transitions,
                implement_gate,
                step_cost,
                &|_c, _m| 0.0,
                explore_routing_orders,
                crit_table,
            );
        }
    }
}

pub fn sabre_solve<
    A: Architecture,
    R: Transition<G, A> + Debug,
    G: GateImplementation + Debug,
    I: IntoIterator<Item = G>,
>(
    c: &Circuit,
    arch: &A,
    transitions: &impl Fn(&Step<G>) -> Vec<R>,
    implement_gate: impl Fn(&Step<G>, &A, &Gate) -> I,
    step_cost: fn(&Step<G>, &A) -> f64,
    mapping_heuristic: Option<fn(&A, &Circuit, &QubitMap) -> f64>,
    explore_routing_orders: bool,
) -> CompilerResult<G> {
    let crit_table = &build_criticality_table(c);
    let mut map = match mapping_heuristic {
        Some(heuristic) => {
            let map_h = |m: &QubitMap| heuristic(arch, c, m);
            let isom_map = isomorphism_map(c, arch);

            let isom_cost = isom_map.clone().map(|x| map_h(&x));
            let sa_map = match isom_cost {
                Some(c) if c == 0.0 => None,
                _ => Some(sim_anneal_mapping_search(
                    random_map(c, arch),
                    arch,
                    INITIAL_TEMP,
                    TERM_TEMP,
                    COOL_RATE,
                    map_h,
                )),
            };
            let sa_cost = sa_map.clone().map(|x| map_h(&x));
            match (isom_cost, sa_cost) {
                (Some(i_c), Some(s_c)) if i_c < s_c => isom_map.unwrap(),
                _ => sa_map.unwrap(),
            }
        }
        None => random_map(c, arch),
    };
    let route_h: Box<dyn Fn(&Circuit, &QubitMap) -> f64> =
        if let Some(ref heuristic) = mapping_heuristic {
            Box::new(|c: &Circuit, m: &QubitMap| heuristic(arch, c, m))
        } else {
            Box::new(|_c: &Circuit, _m: &QubitMap| 0.0)
        };

    for _ in 0..SABRE_ITERATIONS {
        for circ in [c, &c.reversed()] {
            let res = route(
                circ,
                arch,
                map,
                transitions,
                &implement_gate,
                step_cost,
                &route_h,
                explore_routing_orders,
                crit_table,
            );
            map = res.steps.last().unwrap().map.clone();
        }
    }
    return route(
        c,
        arch,
        map,
        transitions,
        &implement_gate,
        step_cost,
        &route_h,
        explore_routing_orders,
        crit_table,
    );
}
