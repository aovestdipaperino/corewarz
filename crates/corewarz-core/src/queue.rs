//! Per-warrior process queues.

use std::collections::VecDeque;

/// A warrior's FIFO queue of process program counters.
///
/// The queue has a fixed capacity; pushing to a full queue is silently
/// ignored, which is how `SPL` becomes a no-op at the process limit.
#[derive(Debug, Clone)]
pub struct ProcessQueue {
    entries: VecDeque<u32>,
    capacity: usize,
}

impl ProcessQueue {
    /// Creates an empty queue holding at most `capacity` processes.
    #[must_use]
    pub fn new(capacity: u32) -> Self {
        Self {
            entries: VecDeque::with_capacity(capacity as usize),
            capacity: capacity as usize,
        }
    }

    /// Enqueues `pc` at the back, unless the queue is full.
    ///
    /// Returns `true` if the process was enqueued.
    pub fn push(&mut self, pc: u32) -> bool {
        if self.entries.len() >= self.capacity {
            return false;
        }
        self.entries.push_back(pc);
        true
    }

    /// Dequeues the next process to run.
    pub fn pop(&mut self) -> Option<u32> {
        self.entries.pop_front()
    }

    /// The process at the front of the queue, without dequeuing it.
    #[must_use]
    pub fn front(&self) -> Option<u32> {
        self.entries.front().copied()
    }

    /// The number of live processes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` if the warrior has no live processes and has lost.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
