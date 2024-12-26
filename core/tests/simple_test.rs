use crossbeam_channel::unbounded;
use rsim_core::sim_dispatcher::SimDispatcher;
use rsim_core::sim_manager::SimManager;
use rsim_core::simple_component::simple_link::SimpleLink;
use rsim_core::simple_component::simple_receiver::SimpleReceiver;
use rsim_core::simple_component::simple_sender::SimpleSender;
use std::sync::Arc;
use std::thread;
use std::time::SystemTime;

#[test]
fn simple_test() {
    let ack_channel = unbounded();

    let sim_manager = SimManager::new(ack_channel.1);

    let input_link_pair = unbounded();
    let output_link_pair = unbounded();

    let link = SimpleLink::new(
        0,
        sim_manager.clone(),
        input_link_pair.1,
        output_link_pair.0,
        ack_channel.0.clone(),
    );

    let sender = SimpleSender::new(
        1,
        sim_manager.clone(),
        100,
        input_link_pair.0,
        ack_channel.0.clone(),
    );

    let receiver = SimpleReceiver::new(
        2,
        sim_manager.clone(),
        output_link_pair.1,
        ack_channel.0.clone(),
    );

    let sim_dispatchers = vec![
        SimDispatcher::new(Arc::downgrade(&sim_manager), vec![sender]),
        SimDispatcher::new(Arc::downgrade(&sim_manager), vec![link]),
        SimDispatcher::new(Arc::downgrade(&sim_manager), vec![receiver]),
    ];

    sim_dispatchers.iter().for_each(|s| s.init());

    let mut thread_handlers = vec![];

    for sim_dispatcher in sim_dispatchers {
        thread_handlers.push(thread::spawn(move || sim_dispatcher.run()));
    }

    let start = SystemTime::now();
    sim_manager.run();
    let processing_time = start.elapsed().unwrap().as_secs_f64();

    thread_handlers.into_iter().for_each(|h| {
        h.join().unwrap();
    });

    let event_processed = sim_manager.get_event_processed();
    println!(
        "Finished processing {} events in {} seconds @ {} events/second",
        event_processed,
        processing_time,
        event_processed as f64 / processing_time
    );
}
