use qmrl::*;
use petgraph::{graph::NodeIndex, Graph};

#[derive(Debug)]
pub struct NisqGateImplementation {
    u: Location,
    v: Location,
}
struct MyArch {
    graph: Graph<Location, ()>,
    index_map: HashMap<Location, NodeIndex>,
}
#[derive(Debug)]
pub struct Swap {
    u: Location,
    v: Location,
}
impl GateImplementation for NisqGateImplementation {}
impl Architecture for MyArch {
    fn get_locations(&self) -> Vec<Location> {
        let mut locations = Vec::new();
        for node in self.graph.node_indices() {
            locations.push(self.graph[node]);
        }
        return locations;
    }
}
impl Transition<NisqGateImplementation> for Swap {
    fn apply(
        &self,
        step: &Step<NisqGateImplementation>,
    ) -> Step<NisqGateImplementation> {
        let mut new_step = step.clone();
        let left = self.u;
        let right = self.v;
        new_step.map = utils::swap_keys(&step.map, left, right);
        return new_step;
    }
    fn repr(&self) -> String {
        return format!("{:?}", self);
    }
    fn cost(&self) -> f64 {
        0f64
    }
}
