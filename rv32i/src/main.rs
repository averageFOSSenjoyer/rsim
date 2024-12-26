use crate::backend::core::Core;
use crate::backend::util::types::Word;
use std::fs;
use std::fs::File;
use std::time::SystemTime;

mod backend;

fn main() {
    let commit_file = std::env::var("LOG_FILE")
        .map(|log_file| File::create(log_file).ok())
        .ok()
        .unwrap_or(None);
    let mut core = Core::new(4, commit_file);

    let data = fs::read(
        std::env::var("BIN_FILE").expect("Provide the path to the binary file in BIN_FILE env var"),
    )
    .unwrap();
    core.load_bin(&data, Word::from(0x40000000u32));

    let start = SystemTime::now();
    core.run_end();
    let processing_time = start.elapsed().unwrap().as_secs_f64();
    let event_processed = core.sim_manager.get_event_processed();
    println!(
        "Finished processing {} events in {} seconds @ {} events/second",
        event_processed,
        processing_time,
        event_processed as f64 / processing_time
    );
}
