use crate::clock_event::ClockEvent;
use crate::error::SimError;
use crate::event::Event;
use crate::task::Task;
use crate::types::Output;
use crate::types::{ComponentId, Cycle, EventId};
use crossbeam_channel::{Receiver, Sender};
use std::collections::binary_heap::BinaryHeap;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub struct SimManager {
    curr_cycle: Mutex<Cycle>,
    event_q: Mutex<BinaryHeap<Task>>,
    clock_tick_q: Mutex<Vec<Output>>,
    rob: Mutex<HashSet<EventId>>,
    next_event_id: Mutex<EventId>,
    ack_recv: Receiver<EventId>,
    component_do_not_end_set: Mutex<HashSet<ComponentId>>,
    event_processed: Mutex<u128>,
}

impl SimManager {
    pub fn new(ack_recv: Receiver<EventId>) -> Arc<Self> {
        Arc::new(SimManager {
            curr_cycle: Mutex::new(0),
            event_q: Mutex::new(BinaryHeap::new()),
            clock_tick_q: Mutex::new(Vec::new()),
            rob: Mutex::new(HashSet::new()),
            next_event_id: Mutex::new(0),
            ack_recv,
            component_do_not_end_set: Mutex::new(HashSet::new()),
            event_processed: Mutex::new(0),
        })
    }

    pub fn enq_event(&self, event: Task) {
        let _ = self.event_q.lock().map(|mut event_q| event_q.push(event));
    }

    pub fn get_curr_cycle(&self) -> Cycle {
        *self.curr_cycle.lock().unwrap()
    }

    fn increment_cycle(&self) {
        let _ = self
            .curr_cycle
            .lock()
            .map(|mut curr_cycle| *curr_cycle += 1);
    }

    pub fn request_new_event_id(&self) -> EventId {
        let mut next_event_id = self.next_event_id.lock().unwrap();
        let ret = *next_event_id;
        *next_event_id += 1;
        ret
    }

    pub fn register_clock_tick(&self, sender: Output) {
        self.clock_tick_q.lock().unwrap().push(sender)
    }

    pub fn register_do_not_end(&self, component_id: ComponentId) {
        let _ = self
            .component_do_not_end_set
            .lock()
            .map(|mut set| set.insert(component_id));
    }

    pub fn register_can_end(&self, component_id: ComponentId) {
        let _ = self
            .component_do_not_end_set
            .lock()
            .map(|mut set| set.remove(&component_id));
    }

    pub fn sim_can_end(&self) -> bool {
        self.component_do_not_end_set
            .lock()
            .map(|set| set.is_empty())
            .unwrap_or(false)
    }

    fn recv_ack(&self) {
        while let Ok(ack_id) = self.ack_recv.try_recv() {
            if let Ok(mut rob) = self.rob.lock() {
                if !rob.remove(&ack_id) {
                    panic!("ack'd non-existing task");
                }
                *self.event_processed.lock().unwrap() += 1;
            }
        }
    }

    pub fn get_event_processed(&self) -> u128 {
        *self.event_processed.lock().unwrap()
    }

    fn send_events(&self) {
        let _ = self.event_q.lock().map(|mut locked_event_q| {
            while let Some(task) = locked_event_q.peek() {
                if task.event.get_scheduled_time() <= self.get_curr_cycle() {
                    if task.event.get_scheduled_time() < self.get_curr_cycle() {
                        panic!("Time fault detected!");
                    }
                    if let Some(task) = locked_event_q.pop() {
                        let _ = self
                            .rob
                            .lock()
                            .map(|mut rob| rob.insert(task.event.get_event_id()));
                        let _ = task.event_callback.try_send(task.event);
                    };
                } else {
                    break;
                }
            }
        });
    }

    fn schedule_clock_tasks(&self) {
        if let Ok(clock_tick_q) = self.clock_tick_q.lock() {
            for clock_tick_task in clock_tick_q.iter() {
                let clock_event =
                    ClockEvent::new(self.get_curr_cycle(), self.request_new_event_id());
                self.event_q
                    .lock()
                    .unwrap()
                    .push(Task::new(Box::new(clock_event), clock_tick_task.clone()));
            }
        }
    }

    fn can_increase_cycle(&self) -> Result<bool, SimError> {
        Ok(self.rob.lock()?.is_empty()
            && (self.event_q.lock()?.is_empty()
                || self
                    .event_q
                    .lock()?
                    .peek()
                    .ok_or(SimError::SimManagerError)?
                    .event
                    .get_scheduled_time()
                    > self.get_curr_cycle()))
    }

    /// For testing comb logics, I don't see what else this is useful for
    pub fn run_cycle_end(&self) -> Result<(), SimError> {
        loop {
            self.recv_ack();
            self.send_events();

            if self.can_increase_cycle()? {
                return Ok(());
            }
        }
    }

    pub fn run_cycle(&self) -> Result<(), SimError> {
        loop {
            self.recv_ack();
            self.send_events();

            // Time to move on to the next cycle
            if self.can_increase_cycle()? {
                self.increment_cycle();
                self.schedule_clock_tasks();
                self.send_events();
                while !self.rob.lock().unwrap().is_empty() && !self.sim_can_end() {
                    // !self.sim_can_end() is needed, not sure why
                    self.recv_ack();
                }
                return Ok(());
            }
        }
    }

    pub fn run(&self) {
        loop {
            let _ = self.run_cycle();

            if self.sim_can_end() {
                break;
            }
        }
    }

    pub fn proxy_event(&self, event: Box<dyn Event>, callback: Sender<Box<dyn Event>>) {
        let mut locked_rob = self.rob.lock().unwrap();
        let task = Task::new(event, callback);
        locked_rob.insert(task.event.get_event_id());
        task.event_callback.try_send(task.event).unwrap();
    }
}
