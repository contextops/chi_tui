use regex::Regex;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use crate::widgets::watchdog::config::WatchdogStatSpec;

#[derive(Clone)]
pub struct Pattern {
    pub label: String,
    pub re: Regex,
}

pub struct StatsAggregator {
    patterns: Vec<Pattern>,
    counts: Vec<usize>,
    last_lens: Vec<usize>,
}

impl StatsAggregator {
    pub fn new(specs: &[WatchdogStatSpec], streams: usize) -> Self {
        let patterns: Vec<Pattern> = specs
            .iter()
            .filter_map(|s| {
                Regex::new(&s.regexp).ok().map(|re| Pattern {
                    label: s.label.clone(),
                    re,
                })
            })
            .collect();
        let counts = vec![0usize; patterns.len()];
        let last_lens = vec![0usize; streams];
        Self {
            patterns,
            counts,
            last_lens,
        }
    }

    pub fn labels(&self) -> Vec<String> {
        self.patterns.iter().map(|p| p.label.clone()).collect()
    }

    pub fn len(&self) -> usize {
        self.patterns.len()
    }

    pub fn counts(&self) -> &[usize] {
        &self.counts
    }

    // Update counts incrementally if buffers only grew; otherwise recompute all.
    pub fn update_from_buffers(&mut self, buffers: &[Arc<Mutex<VecDeque<String>>>]) {
        // If any buffer shrank, do full recompute
        let mut need_full = false;
        for (i, buf) in buffers.iter().enumerate() {
            if let Ok(q) = buf.lock() {
                let cur = q.len();
                if cur < self.last_lens.get(i).copied().unwrap_or(0) {
                    need_full = true;
                    break;
                }
            }
        }
        if need_full {
            self.recompute(buffers);
            return;
        }
        // Incremental update
        for (i, buf) in buffers.iter().enumerate() {
            if let Ok(q) = buf.lock() {
                let cur = q.len();
                let last = self.last_lens.get(i).copied().unwrap_or(0);
                if cur > last {
                    for s in q.iter().skip(last).take(cur - last) {
                        for (pi, pat) in self.patterns.iter().enumerate() {
                            self.counts[pi] =
                                self.counts[pi].saturating_add(pat.re.find_iter(s).count());
                        }
                    }
                    if let Some(slot) = self.last_lens.get_mut(i) {
                        *slot = cur;
                    }
                }
            } else if let Some(slot) = self.last_lens.get_mut(i) {
                *slot = 0;
            }
        }
    }

    fn recompute(&mut self, buffers: &[Arc<Mutex<VecDeque<String>>>]) {
        self.counts.fill(0);
        for (i, buf) in buffers.iter().enumerate() {
            if let Ok(q) = buf.lock() {
                for s in q.iter() {
                    for (pi, pat) in self.patterns.iter().enumerate() {
                        self.counts[pi] =
                            self.counts[pi].saturating_add(pat.re.find_iter(s).count());
                    }
                }
                if let Some(slot) = self.last_lens.get_mut(i) {
                    *slot = q.len();
                }
            } else if let Some(slot) = self.last_lens.get_mut(i) {
                *slot = 0;
            }
        }
    }
}
