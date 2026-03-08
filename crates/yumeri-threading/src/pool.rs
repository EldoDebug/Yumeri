use std::panic::{AssertUnwindSafe, catch_unwind};

use crate::task::{Task, TaskError};

type Job = Box<dyn FnOnce() + Send>;

pub struct ThreadPool {
    sender: Option<crossbeam_channel::Sender<Job>>,
    workers: Vec<std::thread::JoinHandle<()>>,
}

impl ThreadPool {
    pub fn new(num_threads: usize) -> Self {
        assert!(num_threads > 0, "thread pool needs at least 1 thread");

        let (sender, receiver) = crossbeam_channel::bounded(num_threads * 4);
        let mut workers = Vec::with_capacity(num_threads);

        for i in 0..num_threads {
            let rx = receiver.clone();
            let handle = std::thread::Builder::new()
                .name(format!("yumeri-worker-{i}"))
                .spawn(move || {
                    while let Ok(job) = rx.recv() {
                        let _ = catch_unwind(AssertUnwindSafe(job));
                    }
                })
                .expect("failed to spawn worker thread");
            workers.push(handle);
        }

        Self {
            sender: Some(sender),
            workers,
        }
    }

    pub fn with_default_size() -> Self {
        Self::new(crate::parallelism().clamp(2, 16))
    }

    pub fn spawn(&self, f: impl FnOnce() + Send + 'static) {
        let sender = self.sender.as_ref().expect("thread pool shut down");
        sender
            .send(Box::new(f))
            .expect("thread pool channel disconnected");
    }

    pub fn spawn_task<T: Send + 'static>(
        &self,
        f: impl FnOnce() -> T + Send + 'static,
    ) -> Task<T> {
        let (tx, rx) = crossbeam_channel::bounded(1);
        self.spawn(move || {
            let result = catch_unwind(AssertUnwindSafe(f));
            let _ = tx.send(match result {
                Ok(v) => Ok(v),
                Err(panic) => Err(panic_to_error(panic)),
            });
        });
        Task::new(rx)
    }

    pub fn thread_count(&self) -> usize {
        self.workers.len()
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        self.sender.take();
        for handle in self.workers.drain(..) {
            let _ = handle.join();
        }
    }
}

fn panic_to_error(payload: Box<dyn std::any::Any + Send>) -> TaskError {
    let msg = if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown panic".to_string()
    };
    TaskError::Panicked(msg)
}
