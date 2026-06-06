//! # construct-supply-chain
//!
//! Constructs flow from git repos through validation, compilation, and deployment.
//! Each stage has a queue, rejection rate, and throughput metrics.

use std::collections::VecDeque;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stage { Discovered, Validating, Compiling, Deploying, Live, Rejected, Cached }

#[derive(Debug, Clone)]
pub struct Construct {
    pub name: String,
    pub version: String,
    pub stage: Stage,
    pub size_bytes: usize,
    pub validation_score: f64,
    pub compile_time_us: u64,
}

pub struct SupplyChain {
    discovered: VecDeque<Construct>,
    validating: VecDeque<Construct>,
    compiling: VecDeque<Construct>,
    deploying: VecDeque<Construct>,
    live: VecDeque<Construct>,
    rejected: VecDeque<Construct>,
    cached: VecDeque<Construct>,
    stats: ChainStats,
}

#[derive(Debug, Clone, Default)]
pub struct ChainStats {
    pub total_discovered: u64,
    pub total_validated: u64,
    pub total_compiled: u64,
    pub total_deployed: u64,
    pub total_rejected: u64,
    pub total_time_us: u64,
}

impl SupplyChain {
    pub fn new() -> Self {
        Self {
            discovered: VecDeque::new(), validating: VecDeque::new(),
            compiling: VecDeque::new(), deploying: VecDeque::new(),
            live: VecDeque::new(), rejected: VecDeque::new(), cached: VecDeque::new(),
            stats: ChainStats::default(),
        }
    }

    pub fn discover(&mut self, name: &str, version: &str, size: usize) {
        self.discovered.push_back(Construct {
            name: name.into(), version: version.into(), stage: Stage::Discovered,
            size_bytes: size, validation_score: 0.0, compile_time_us: 0,
        });
        self.stats.total_discovered += 1;
    }

    /// Advance one construct through the pipeline.
    pub fn advance(&mut self) -> Option<String> {
        // Try each stage
        if let Some(mut c) = self.discovered.pop_front() {
            c.stage = Stage::Validating;
            c.validation_score = 0.9; // simulated
            if c.validation_score > 0.5 {
                self.validating.push_back(c);
                self.stats.total_validated += 1;
                return Some("discovered→validating".into());
            } else {
                c.stage = Stage::Rejected;
                self.rejected.push_back(c);
                self.stats.total_rejected += 1;
                return Some("discovered→rejected".into());
            }
        }
        if let Some(mut c) = self.validating.pop_front() {
            c.stage = Stage::Compiling;
            c.compile_time_us = (c.size_bytes as u64 / 100).max(10);
            self.stats.total_time_us += c.compile_time_us;
            self.compiling.push_back(c);
            self.stats.total_compiled += 1;
            return Some("validating→compiling".into());
        }
        if let Some(mut c) = self.compiling.pop_front() {
            c.stage = Stage::Deploying;
            self.deploying.push_back(c);
            return Some("compiling→deploying".into());
        }
        if let Some(mut c) = self.deploying.pop_front() {
            c.stage = Stage::Live;
            self.stats.total_deployed += 1;
            self.live.push_back(c);
            return Some("deploying→live".into());
        }
        None
    }

    /// Run the full pipeline until empty.
    pub fn process_all(&mut self) -> Vec<String> {
        let mut transitions = Vec::new();
        while let Some(t) = self.advance() { transitions.push(t); }
        transitions
    }

    pub fn live_count(&self) -> usize { self.live.len() }
    pub fn rejected_count(&self) -> usize { self.rejected.len() }
    pub fn queue_depth(&self) -> usize {
        self.discovered.len() + self.validating.len() + self.compiling.len() + self.deploying.len()
    }
    pub fn stats(&self) -> &ChainStats { &self.stats }
    pub fn throughput(&self) -> f64 {
        if self.stats.total_time_us == 0 { return 0.0; }
        self.stats.total_deployed as f64 / (self.stats.total_time_us as f64 / 1_000_000.0)
    }

    /// Rejection rate across the pipeline.
    pub fn rejection_rate(&self) -> f64 {
        if self.stats.total_discovered == 0 { return 0.0; }
        self.stats.total_rejected as f64 / self.stats.total_discovered as f64
    }
}

impl Default for SupplyChain {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover() {
        let mut sc = SupplyChain::new();
        sc.discover("attention", "v1", 1024);
        assert_eq!(sc.queue_depth(), 1);
        assert_eq!(sc.stats().total_discovered, 1);
    }

    #[test]
    fn test_full_pipeline() {
        let mut sc = SupplyChain::new();
        sc.discover("kernel-a", "v1", 500);
        let transitions = sc.process_all();
        assert!(transitions.len() >= 4); // 4 stages
        assert_eq!(sc.live_count(), 1);
        assert_eq!(sc.rejected_count(), 0);
    }

    #[test]
    fn test_multiple_constructs() {
        let mut sc = SupplyChain::new();
        for i in 0..5 { sc.discover(&format!("k{}", i), "v1", 100 * (i + 1)); }
        sc.process_all();
        assert_eq!(sc.live_count(), 5);
        assert_eq!(sc.queue_depth(), 0);
    }

    #[test]
    fn test_stats() {
        let mut sc = SupplyChain::new();
        sc.discover("a", "v1", 100);
        sc.process_all();
        assert_eq!(sc.stats().total_discovered, 1);
        assert_eq!(sc.stats().total_validated, 1);
        assert_eq!(sc.stats().total_compiled, 1);
        assert_eq!(sc.stats().total_deployed, 1);
    }

    #[test]
    fn test_throughput() {
        let mut sc = SupplyChain::new();
        for i in 0..10 { sc.discover(&format!("k{}", i), "v1", 1000); }
        sc.process_all();
        assert!(sc.throughput() > 0.0);
    }

    #[test]
    fn test_rejection_rate() {
        let mut sc = SupplyChain::new();
        sc.discover("good", "v1", 100);
        sc.process_all();
        assert_eq!(sc.rejection_rate(), 0.0);
    }
}
