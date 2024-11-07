use rand::seq::SliceRandom;

use crate::utils::{Architecture, Circuit, Location, Qubit, Step, Transition};
use std::collections::HashMap;
const ALPHA: f64 = 1.0 / 3.0;
const BETA: f64 = 1.0 / 3.0;
const GAMMA: f64 = 2.0 / 3.0;

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

fn simulated_anneal<T: Copy>(
    start: T,
    initial_temp: f64,

    term_temp: f64,
    cool_rate: f64,
    random_neighbor: fn(T) -> T,
    cost_function: fn(T) -> f64,
) -> T {
    let mut current = start;
    let mut temp = initial_temp;
    let mut best = start;
    let mut best_cost = cost_function(start);
    while temp > term_temp {
        let next = random_neighbor(current);
        let next_cost = cost_function(next);
        let delta = next_cost - best_cost;
        let rand: f64 = rand::random();
        if delta < 0.0 || (delta > 0.0 && rand < (-delta / temp).exp()) {
            current = next;
            best = next;
            best_cost = next_cost;
        }
        temp *= cool_rate;
    }
    return best;
}

fn route<T: Architecture, R: Transition>(
    c: &Circuit,
    arch: &T,
    map: HashMap<Qubit, Location>,
    transitions: &Vec<R>,
    valid_step: fn(&Step, &T) -> bool,
    step_cost: fn(&Step) -> f64,
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
        let best = find_best_next_step(
            &current_circ,
            arch,
            &transitions,
            valid_step,
            steps.last(),
            step_cost,
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
) -> Option<(Step, &'a R, f64)> {
    let mut best: Option<(Step, &R, f64)> = None;
    for trans in transitions {
        let mut next_step = trans.apply(last_step.unwrap());
        let executable = c.get_front_layer();
        next_step.maximize_step(&executable, arch, valid_step);
        let s_cost = step_cost(&next_step);
        let t_cost = trans.cost();

        let cost = ALPHA * s_cost + BETA * t_cost + GAMMA * -(next_step.gates.len() as f64);
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
) -> (Vec<Step>, Vec<String>, f64) {
    let map = random_map(c, arch);
    return route(c, arch, map, transitions, valid_step, step_cost);
}
