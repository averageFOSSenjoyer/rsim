use crate::component::Component;
use crate::sim_manager::SimManager;
use std::sync::{Arc, Mutex, Weak};

pub struct SimDispatcher {
    sim_manager: Weak<SimManager>,
    components: Vec<Arc<Mutex<dyn Component>>>,
}

impl SimDispatcher {
    pub fn new(
        sim_manager: Weak<SimManager>,
        components: Vec<Arc<Mutex<dyn Component>>>,
    ) -> Arc<Self> {
        Arc::new(SimDispatcher {
            sim_manager,
            components,
        })
    }

    /// `init` is called prior to the start of the simulation.
    /// This function in turns calls the `init` of all its child components.
    /// see `crate::component::Component::init`
    pub fn init(self: &Arc<Self>) {
        for component in self.components.iter() {
            component.lock().unwrap().init()
        }
    }

    pub fn run(self: &Arc<Self>) {
        loop {
            for component in self.components.iter() {
                component.lock().unwrap().poll_recv()
            }
            if self.sim_manager.upgrade().unwrap().sim_can_end() {
                break;
            }
        }
    }
}
