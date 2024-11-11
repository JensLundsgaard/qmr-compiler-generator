use rand::seq::SliceRandom;

use crate::utils::*;
use std::collections::HashMap;
const ALPHA: f64 = 1.0;
const BETA: f64 = 1.0;
const GAMMA: f64 = 2.0;
const DELTA: f64 = 2.0;
fn random_map<T: Architecture>(c: &Circuit, arch: &T) -> HashMap<Qubit, Location> {
    let mut map = HashMap::new();
    let mut rng = &mut rand::thread_rng();
    let locations = arch.get_locations();
    let v = locations.choose_multiple(&mut rng, c.qubits.len());
    for (q, l) in c.qubits.iter().zip(v) {
        map.insert(*q, **l);
    }
    return map;
}

fn simulated_anneal<T: Clone>(
    start: T,
    initial_temp: f64,

    term_temp: f64,
    cool_rate: f64,
    random_neighbor: impl Fn(&T) -> T,
    cost_function: impl Fn(&T) -> f64,
) -> T {
    let mut best = start.clone();
    let mut best_cost = cost_function(&best);
    let mut current = start.clone();
    let mut curr_cost = cost_function(&current);
    let mut temp = initial_temp;
    while temp > term_temp {
        let next = random_neighbor(&current);
        let next_cost = cost_function(&next);
        let delta_curr = next_cost - curr_cost;
        let delta_best = next_cost - best_cost;
        let rand: f64 = rand::random();
        if delta_best < 0.0 {
            best = next.clone();
            best_cost = next_cost;
            current = next;
            curr_cost = next_cost;
        } else if rand < (-delta_curr / temp).exp() {
            current = next;
            curr_cost = next_cost;
        }
        temp *= cool_rate;
    }
    return best;
}

fn random_neighbor<T: Architecture>(
    map: &HashMap<Qubit, Location>,
    arch: &T,
) -> HashMap<Qubit, Location> {
    let mut moves: Vec<Box<dyn Fn(&HashMap<Qubit, Location>) -> HashMap<Qubit, Location>>> =
        Vec::new();
    for q1 in map.keys() {
        for q2 in map.keys() {
            if q1 == q2 {
                continue;
            }
            let swap_keys = |m: &HashMap<Qubit, Location>| {
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
        for l in arch.get_locations() {
            if !map.values().any(|x| x == l) {
                let into_open = |m: &HashMap<Qubit, Location>| {
                    let mut new_map = m.clone();
                    new_map.insert(*q, *l);
                    return new_map;
                };
                moves.push(Box::new(into_open));
            }
        }
    }
    let rng = &mut rand::thread_rng();
    let chosen_move = moves.choose(rng).unwrap();
    return chosen_move(&map);
}

fn sim_anneal_mapping_search<T: Architecture>(
    start: HashMap<Qubit, Location>,
    arch: &T,
    initial_temp: f64,
    term_temp: f64,
    cool_rate: f64,
    heuristic: impl Fn(&HashMap<Qubit, Location>) -> f64,
) -> HashMap<Qubit, Location> {
    return simulated_anneal(
        start,
        initial_temp,
        term_temp,
        cool_rate,
        |m| random_neighbor(m, arch),
        heuristic,
    );
}

fn route<T: Architecture, R: Transition>(
    c: &Circuit,
    arch: &T,
    map: HashMap<Qubit, Location>,
    transitions: &Vec<R>,
    valid_step: fn(&Step, &T) -> bool,
    step_cost: fn(&Step) -> f64,
    map_eval: impl Fn(&Circuit, &HashMap<Qubit, Location>) -> f64,
) -> (Vec<Step>, Vec<String>, f64) {
    let mut steps = Vec::new();
    let mut trans_taken = Vec::new();
    let mut step_0 = Step {
        gates: Vec::new(),
        map,
    };
    let mut current_circ = c.clone();
    let mut cost = 0.0;
    let executable = &c.get_front_layer();
    step_0.maximize_step(executable, arch, valid_step);
    current_circ.remove_gates(&(step_0.gates));
    steps.push(step_0);
    while current_circ.gates.len() > 0 {
        println!("{:?}", current_circ.gates.len());
        let best = find_best_next_step(
            &current_circ,
            arch,
            &transitions,
            valid_step,
            steps.last(),
            step_cost,
            &map_eval,
        );
        match best {
            Some((s, trans, _b)) => {
                current_circ.remove_gates(&s.gates);
                steps.push(s);
                trans_taken.push(trans.repr());
                cost += trans.cost();
            }
            None => {
                panic!("No valid next step found");
            }
        }
    }
    return (steps, trans_taken, cost);
}

fn find_best_next_step<'a, T: Architecture, R: Transition>(
    c: &Circuit,
    arch: &T,
    transitions: &'a Vec<R>,
    valid_step: fn(&Step, &T) -> bool,
    last_step: Option<&Step>,
    step_cost: fn(&Step) -> f64,
    map_eval: impl Fn(&Circuit, &HashMap<Qubit, Location>) -> f64,
) -> Option<(Step, &'a R, f64)> {
    let mut best: Option<(Step, &R, f64)> = None;
    for trans in transitions {
        let mut next_step = trans.apply(last_step.unwrap());
        let executable = c.get_front_layer();
        next_step.maximize_step(&executable, arch, valid_step);
        let s_cost = step_cost(&next_step);
        let t_cost = trans.cost();
        let m_cost = map_eval(&circuit_from_gates(executable), &next_step.map);
        let weighted_vals = std::iter::zip(
            vec![ALPHA, BETA, GAMMA, DELTA],
            vec![s_cost, t_cost, m_cost, -(next_step.gates.len() as f64)],
        );
        let cost = drop_zeros_and_normalize(weighted_vals);
        match best {
            Some((ref _s, _prev_trans, b)) => {
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

pub fn solve<T: Architecture, R: Transition>(
    c: &Circuit,
    arch: &T,
    transitions: &Vec<R>,
    valid_step: fn(&Step, &T) -> bool,
    step_cost: fn(&Step) -> f64,
    mapping_heuristic: Option<fn(&T, &Circuit, &HashMap<Qubit, Location>) -> f64>,
) -> (Vec<Step>, Vec<String>, f64) {
    match mapping_heuristic {
        Some(heuristic) => {
            let map_h = |m: &HashMap<Qubit, Location>| heuristic(arch, c, m);
            let route_h = |c: &Circuit, m: &HashMap<Qubit, Location>| heuristic(arch, c, m);
            let map =
                sim_anneal_mapping_search(random_map(c, arch), arch, 1000.0, 0.0001, 0.99, map_h);
            println!("{:?}", map);
            return route(c, arch, map, transitions, valid_step, step_cost, route_h);
        }
        None => {
            let map = random_map(c, arch);
            return route(
                c,
                arch,
                map,
                transitions,
                valid_step,
                step_cost,
                |_c, _m| 0.0,
            );
        }
    }
}
