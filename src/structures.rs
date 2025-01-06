use std::{collections::{HashMap, HashSet}, fmt};
use serde::Serialize;

#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug, Serialize)]
pub struct Qubit(usize);
impl Qubit {
    pub fn new(i: usize) -> Self {
        return Qubit(i);
    }
    pub fn get_index(&self) -> usize {
        return self.0;
    }
}

#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug, Default, Serialize)]
pub struct Location(usize);

pub type QubitMap = HashMap<Qubit, Location>;

impl Location {
    pub fn new(i: usize) -> Self {
        return Location(i);
    }
    pub fn get_index(&self) -> usize {
        return self.0;
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum GateType {
    CX,
    T,
}
impl fmt::Display for GateType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GateType::CX => write!(f, "CX"),
            GateType::T => write!(f, "T"),
            
        }
    }
}



#[derive(Clone, Debug, Eq, Hash)]
pub struct Gate {
    pub gate_type : GateType,
    pub qubits: Vec<Qubit>,
    pub id: usize,
}
impl Serialize for Gate {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&format!("{:?} {:?}", self.gate_type, self.qubits))
    }
}

#[derive(Clone, Debug)]
pub struct Circuit {
    pub gates: Vec<Gate>,
    pub qubits: HashSet<Qubit>,
}

impl PartialEq for Gate {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Circuit {
    pub fn get_front_layer(&self) -> Vec<Gate> {
        let mut blocked_qubits: HashSet<Qubit> = HashSet::new();
        let mut gates = Vec::new();
        for g in &self.gates {
            let gate_qubits = &g.qubits;
            let not_blocked = gate_qubits.iter().all(|q| !blocked_qubits.contains(q));
            if not_blocked {
                gates.push(g.clone());
            }
            blocked_qubits.extend(gate_qubits);
        }
        return gates;
    }
    pub fn remove_gates(&mut self, gates: &Vec<Gate>) {
        self.gates.retain(|g| !gates.contains(g));
    }
    pub fn reversed(&self)-> Circuit {
        let mut copy =  self.clone();
        copy.gates.reverse();
        return copy;
    }
}

pub fn circuit_from_gates(gates: Vec<Gate>) -> Circuit {
    let mut qubits = HashSet::new();
    for gate in &gates {
        for qubit in &gate.qubits {
            qubits.insert(*qubit);
        }
    }
    return Circuit { gates, qubits };
}

pub trait GateImplementation {}

#[derive(Clone, Debug, Serialize)]
pub struct Step<T: GateImplementation> {
    pub map: QubitMap,
    pub implementation: HashMap<Gate, T>,
}

impl<G: GateImplementation> Step<G> {
    pub fn max_step<A: Architecture>(
        &mut self,
        executable: &Vec<Gate>,
        arch: &A,
        implement_gate: impl Fn(&Step<G>, &A, &Gate) -> Option<G>,
    ) {
        assert!(self.implementation.is_empty());
        for gate in executable {
            let implementation = implement_gate(self, arch, gate);
            match implementation {
                None => continue,
                Some(implementation) => {
                    self.implementation.insert(gate.clone(), implementation);
                }
            }
        }
    }

    pub fn gates(&self) -> Vec<Gate> {
        return self.implementation.keys().cloned().collect();
    }
}

pub trait Transition<T: GateImplementation> {
    fn apply(&self, step: &Step<T>) -> Step<T>;
    fn repr(&self) -> String;
    fn cost(&self) -> f64;
}

pub trait Architecture {
    fn get_locations(&self) -> Vec<Location>;
}

#[derive(Serialize, Debug)]
pub struct CompilerResult<T : GateImplementation> {
    pub steps : Vec<Step<T>>,
    pub transitions : Vec<String>,
    pub cost : f64,

}