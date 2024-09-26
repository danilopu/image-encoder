use std::time::{Instant, Duration};
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use chrono;
use parking_lot::Mutex;

pub struct Logger {
    sender: mpsc::Sender<String>,
}

impl Logger {
    pub fn new(log_messages: Arc<Mutex<Vec<String>>>) -> Self {
        let (sender, receiver) = mpsc::channel();
        
        thread::spawn(move || {
            for message in receiver {
                log_messages.lock().push(message);
            }
        });

        Logger { sender }
    }

    pub fn log(&self, message: String) {
        let timestamp = chrono::Local::now().format("%H:%M:%S%.3f");
        let log_message = format!("[{}] {}", timestamp, message);
        self.sender.send(log_message).unwrap_or_default();
    }
}

pub fn measure_time<F, T>(f: F) -> (T, Duration)
where
    F: FnOnce() -> T,
{
    let start = Instant::now();
    let result = f();
    let duration = start.elapsed();
    (result, duration)
}

pub fn get_memory_usage() -> String {
    if let Ok(mem_info) = sys_info::mem_info() {
        format!(
            "Memory: Total: {} MB, Free: {} MB, Used: {} MB",
            mem_info.total / 1024,
            mem_info.free / 1024,
            (mem_info.total - mem_info.free) / 1024
        )
    } else {
        "Unable to get memory info".to_string()
    }
}
