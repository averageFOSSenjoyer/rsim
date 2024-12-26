#[macro_export]
macro_rules! ack {
    ($self:ident, $event_id:expr) => {
        $self.ack_sender.send($event_id).unwrap()
    };
}

#[macro_export]
macro_rules! enq {
    ($self:ident, $task:expr) => {
        $self.sim_manager.enq_event($task)
    };
}

#[macro_export]
macro_rules! send {
    ($self:ident, $output:expr, $event:expr) => {
        let task = Task::new(Box::new($event), $output.clone());
        enq!($self, task);
    };
}
