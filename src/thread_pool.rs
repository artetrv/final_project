use std::sync::{mpsc, Arc, Mutex};
use std::thread;

type Job = Box<dyn FnOnce() + Send + 'static>;

enum Message {
    NewJob(Job),
    Terminate,
}

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Message>,
    receiver: Arc<Mutex<mpsc::Receiver<Message>>>,
}

impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel::<Message>();
        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);
        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        ThreadPool { workers, sender, receiver }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let _ = self.sender.send(Message::NewJob(Box::new(f)));
    }

    pub fn resize(&mut self, new_size: usize) {
        assert!(new_size > 0);
        let current = self.workers.len();

        if new_size > current {
           
            for id in current..new_size {
                self.workers.push(Worker::new(id, Arc::clone(&self.receiver)));
            }
        } else if new_size < current {
            
            let to_remove = current - new_size;

            for _ in 0..to_remove {
                let _ = self.sender.send(Message::Terminate);
            }

            for _ in 0..to_remove {
                if let Some(mut w) = self.workers.pop() {
                    w.join();
                }
            }
        }
    }

    pub fn shutdown(&mut self) {
        for _ in &self.workers {
            let _ = self.sender.send(Message::Terminate);
        }
        while let Some(mut w) = self.workers.pop() {
            w.join();
        }
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        self.shutdown();
    }
}

struct Worker {
    handle: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(_id: usize, receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Worker {
        let handle = thread::spawn(move || loop {
            let message = {
                let rx = receiver.lock().unwrap();
                rx.recv()
            };

            match message {
                Ok(Message::NewJob(job)) => job(),
                Ok(Message::Terminate) | Err(_) => break,
            }
        });

        Worker { handle: Some(handle) }
    }

    fn join(&mut self) {
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn pool_runs_all_jobs() {
        let mut pool = ThreadPool::new(4);
        let counter = Arc::new(Mutex::new(0usize));

        for _ in 0..10 {
            let counter = Arc::clone(&counter);
            pool.execute(move || {
                let mut c = counter.lock().unwrap();
                *c += 1;
            });
        }

        pool.shutdown();
        assert_eq!(*counter.lock().unwrap(), 10);
    }

    #[test]
    fn pool_resize_up_and_down() {
        let mut pool = ThreadPool::new(2);
        pool.resize(6);
        assert_eq!(pool.workers.len(), 6);
        pool.resize(3);
        assert_eq!(pool.workers.len(), 3);
    }
}
