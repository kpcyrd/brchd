use crate::errors::*;
use crate::queue::{Item, QueueClient};

pub struct Standalone {
    destination: String,
}

impl QueueClient for Standalone {
    fn push_work(&mut self, task: Item) -> Result<()> {
        println!("copy item {:?} to {:?}", task, self.destination);
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
