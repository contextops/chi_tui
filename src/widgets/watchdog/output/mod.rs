use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use crate::widgets::watchdog::config::MAX_LINES_PER_CMD;

// Simple output sink backed by a shared VecDeque buffer with max size.
pub struct RingBufferSink {
    buf: Arc<Mutex<VecDeque<String>>>,
}

pub mod stats;

impl RingBufferSink {
    pub fn new(buf: Arc<Mutex<VecDeque<String>>>) -> Self {
        Self { buf }
    }

    pub fn push_line(&self, s: String) {
        if let Ok(mut q) = self.buf.lock() {
            q.push_back(s);
            if q.len() > MAX_LINES_PER_CMD {
                let excess = q.len() - MAX_LINES_PER_CMD;
                for _ in 0..excess {
                    let _ = q.pop_front();
                }
            }
        }
    }

    #[allow(dead_code)]
    pub fn clear(&self) {
        if let Ok(mut q) = self.buf.lock() {
            q.clear();
        }
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        if let Ok(q) = self.buf.lock() {
            q.len()
        } else {
            0
        }
    }
}
