use std::io::{self, Write};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub struct Spinner {
    message: String,
    spinner: Vec<char>,
    delay: u64,
    stop_flag: Arc<AtomicBool>, // Shared flag to signal stopping
}

impl Spinner {
    pub fn new(message: String, spinner: Vec<char>, delay: u64) -> Spinner {
        Spinner {
            message,
            spinner,
            delay,
            stop_flag: Arc::new(AtomicBool::new(false)), 
        }
    }

    pub fn start_spin(&self) -> (JoinHandle<()>, Arc<AtomicBool>) {
        let message = self.message.clone();
        let spinner = self.spinner.clone();
        let delay = self.delay;
        let stop_flag = Arc::clone(&self.stop_flag);

        // Spawn a thread to run the spinner
        let handle = thread::spawn(move || {
            for c in spinner.iter().cycle() {
                if stop_flag.load(Ordering::Relaxed) {
                    break; 
                }
                print!("\r{} {}", message, c); 
                io::stdout().flush().unwrap(); // Ensure immediate output
                thread::sleep(Duration::from_millis(delay));
            }
            print!("\r{}", " ".repeat(message.len() + 2)); // Clear the line
            io::stdout().flush().unwrap();
            println!(); // Newline after stopping
        });

        (handle, Arc::clone(&self.stop_flag)) // Return thread handle and stop flag
    }

    pub fn stop(&self) {
        self.stop_flag.store(true, Ordering::Relaxed); // Signal the spinner to stop
    }
}