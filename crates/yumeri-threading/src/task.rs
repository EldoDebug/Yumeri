use std::fmt;

use crossbeam_channel::Receiver;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    Ready,
    Failed,
}

#[derive(Debug)]
pub enum TaskError {
    Panicked(String),
    Disconnected,
}

impl fmt::Display for TaskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskError::Panicked(msg) => write!(f, "task panicked: {msg}"),
            TaskError::Disconnected => write!(f, "task channel disconnected"),
        }
    }
}

impl std::error::Error for TaskError {}

pub struct Task<T> {
    receiver: Option<Receiver<Result<T, TaskError>>>,
    result: Option<Result<T, TaskError>>,
}

impl<T> Task<T> {
    pub(crate) fn new(receiver: Receiver<Result<T, TaskError>>) -> Self {
        Self {
            receiver: Some(receiver),
            result: None,
        }
    }

    pub fn poll(&mut self) {
        if self.result.is_some() {
            return;
        }
        let Some(rx) = &self.receiver else { return };
        match rx.try_recv() {
            Ok(result) => {
                self.result = Some(result);
                self.receiver = None;
            }
            Err(crossbeam_channel::TryRecvError::Empty) => {}
            Err(crossbeam_channel::TryRecvError::Disconnected) => {
                self.result = Some(Err(TaskError::Disconnected));
                self.receiver = None;
            }
        }
    }

    pub fn try_get(&mut self) -> Option<&T> {
        self.poll();
        self.result.as_ref().and_then(|r| r.as_ref().ok())
    }

    pub fn take(&mut self) -> Option<T> {
        self.poll();
        match self.result.take() {
            Some(Ok(v)) => Some(v),
            other => {
                self.result = other;
                None
            }
        }
    }

    pub fn status(&self) -> TaskStatus {
        match &self.result {
            Some(Ok(_)) => TaskStatus::Ready,
            Some(Err(_)) => TaskStatus::Failed,
            None => TaskStatus::Pending,
        }
    }

    pub fn wait(mut self) -> Result<T, TaskError> {
        if let Some(result) = self.result.take() {
            return result;
        }
        if let Some(rx) = self.receiver.take() {
            return rx.recv().map_err(|_| TaskError::Disconnected)?;
        }
        Err(TaskError::Disconnected)
    }

    pub fn is_ready(&self) -> bool {
        matches!(&self.result, Some(Ok(_)))
    }
}
