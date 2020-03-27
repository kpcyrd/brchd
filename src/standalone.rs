use crate::errors::*;
use crate::queue::{Task, QueueClient};

pub struct Standalone {
    destination: String,
}

impl QueueClient for Standalone {
    fn push_work(&mut self, task: Task) -> Result<()> {
        println!("copy task {:?} to {:?}", task, self.destination);
        // write(&mut self.stream, &IpcMessage::Queue(task))
        Ok(())
    }
}

impl Standalone {
    pub fn new(destination: String) -> Standalone {
        Standalone {
            destination,
        }
    }
}
