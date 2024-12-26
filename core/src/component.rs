use crate::types::ComponentId;
use std::any::Any;

pub trait Component: Send + Sync {
    /// `init` is called prior to the start of the simulation.
    /// This is the time for the components to configure itself with the simulation manager,
    /// such as setting clock tick handlers and sim end hold.
    fn init(&mut self);

    fn reset(&mut self);

    /// `poll_recv` is continuously being called by the component's dispatcher.
    /// The function should not block for a prolonged period of time.
    fn poll_recv(&mut self);

    fn get_component_id(&self) -> ComponentId;

    fn as_any(&self) -> &dyn Any;
}
