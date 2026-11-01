//! Substrate Event Observer: Decoupled observability for real-time verification feedback.
//!
//! Provides a strongly-typed event pipeline enabling terminal UI and external consumers
//! to observe substrate milestones, tool executions, and gate transitions as they occur,
//! without compromising substrate determinism or performance.

use crate::engine::GateVerdict;
use crate::fsm::Phase;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Semantic execution events emitted by the verification engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SubstrateEvent {
    /// A new phase machine lifecycle step has begun.
    PhaseStarted { phase: Phase, description: String },
    /// A phase has concluded with duration.
    PhaseCompleted { phase: Phase, duration_ms: u64 },
    /// An external subprocess or system tool was launched.
    ToolInvoked { tool: String, command: String },
    /// An external tool has finished running.
    ToolCompleted {
        tool: String,
        duration_ms: u64,
        success: bool,
    },
    /// DOM grounding evaluation scored a candidate selector.
    GroundingCandidate {
        selector: String,
        score: f64,
        margin: f64,
    },
    /// A verification gate (Gate 1 or Gate 2) was evaluated.
    GateEvaluated {
        gate: u8,
        verdict: GateVerdict,
        duration_ms: u64,
    },
    /// Autonomous repair cycle initiated an attempt.
    RepairAttempt {
        attempt: u32,
        max_attempts: u32,
        model: String,
    },
    /// Preflight syntax validation succeeded.
    SyntaxCheckPassed { target_path: String },
    /// Live CDP, console error, or network roundtrip telemetry event.
    CdpTelemetry {
        category: String,
        summary: String,
        timestamp_ms: u64,
    },
    /// Gate 3 Stress & Concurrency Quorum evaluation completed.
    StressQuorumEvaluated {
        quorum_passed: bool,
        flaky_detected: bool,
        duration_ms: u64,
    },
    /// Informational or status diagnostic message.
    StatusMessage { message: String },
}

/// Thread-safe observer trait receiving live substrate events.
pub trait SubstrateObserver: std::fmt::Debug + Send + Sync {
    fn on_event(&self, event: SubstrateEvent);
}

/// No-op observer for deterministic CLI or benchmark runs where dynamic rendering is disabled.
#[derive(Debug, Default, Clone, Copy)]
pub struct NullObserver;

impl SubstrateObserver for NullObserver {
    fn on_event(&self, _event: SubstrateEvent) {}
}

/// Channel-based observer that dispatches events over an `std::sync::mpsc::Sender`.
#[derive(Debug, Clone)]
pub struct ChannelObserver {
    sender: std::sync::mpsc::Sender<SubstrateEvent>,
}

impl ChannelObserver {
    #[must_use]
    pub fn new(sender: std::sync::mpsc::Sender<SubstrateEvent>) -> Self {
        Self { sender }
    }
}

impl SubstrateObserver for ChannelObserver {
    fn on_event(&self, event: SubstrateEvent) {
        let _ = self.sender.send(event);
    }
}

/// Multiplexing observer that fans out events to multiple registered observers.
#[derive(Default, Clone)]
pub struct BroadcastObserver {
    listeners: Vec<Arc<dyn SubstrateObserver>>,
}

impl std::fmt::Debug for BroadcastObserver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BroadcastObserver")
            .field("listener_count", &self.listeners.len())
            .finish()
    }
}

impl BroadcastObserver {
    #[must_use]
    pub fn new() -> Self {
        Self {
            listeners: Vec::new(),
        }
    }

    pub fn add_listener(&mut self, listener: Arc<dyn SubstrateObserver>) {
        self.listeners.push(listener);
    }
}

impl SubstrateObserver for BroadcastObserver {
    fn on_event(&self, event: SubstrateEvent) {
        for l in &self.listeners {
            l.on_event(event.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc::channel;

    #[derive(Debug)]
    struct TestCounterObserver(Arc<AtomicUsize>);

    impl SubstrateObserver for TestCounterObserver {
        fn on_event(&self, _event: SubstrateEvent) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn test_event_serialization_roundtrip() {
        let event = SubstrateEvent::PhaseStarted {
            phase: Phase::Gate1Red,
            description: "Evaluating Gate 1 Causal Reproduction".to_string(),
        };
        let serialized = serde_json::to_string(&event).expect("must serialize");
        let deserialized: SubstrateEvent =
            serde_json::from_str(&serialized).expect("must deserialize");
        assert_eq!(event, deserialized);
    }

    #[test]
    fn test_broadcast_and_channel_observer() {
        let (tx, rx) = channel();
        let channel_obs = Arc::new(ChannelObserver::new(tx));
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_obs = Arc::new(TestCounterObserver(Arc::clone(&counter)));

        let mut broadcaster = BroadcastObserver::new();
        broadcaster.add_listener(channel_obs);
        broadcaster.add_listener(counter_obs);

        broadcaster.on_event(SubstrateEvent::StatusMessage {
            message: "Testing broadcast".to_string(),
        });

        assert_eq!(counter.load(Ordering::SeqCst), 1);
        let received = rx.recv().expect("must receive event");
        match received {
            SubstrateEvent::StatusMessage { message } => {
                assert_eq!(message, "Testing broadcast");
            }
            _ => panic!("unexpected event variant"),
        }
    }
}
