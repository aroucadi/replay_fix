//! Guarded phase machine for the dual-gate repair loop.

pub mod matrix;

use crate::error::{CoreError, Result};
pub use matrix::{
    CausalConfidence, DifferentialDiagnosisReport, DifferentialMatrix, DifferentialState,
    ReproExecutionResult,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Phase {
    Init,
    Gate1Red,
    Repair,
    Gate2Green,
    Rejected,
}

impl Phase {
    pub const fn is_terminal(self) -> bool {
        matches!(self, Phase::Gate2Green | Phase::Rejected)
    }

    fn allows(self, next: Phase) -> bool {
        matches!(
            (self, next),
            (Phase::Init, Phase::Gate1Red)
                | (Phase::Gate1Red, Phase::Repair)
                | (Phase::Repair, Phase::Gate2Green)
                | (
                    Phase::Init | Phase::Gate1Red | Phase::Repair,
                    Phase::Rejected
                )
        )
    }
}

/// Records every phase visited and refuses illegal transitions.
#[derive(Debug, Clone)]
pub struct PhaseMachine {
    current: Phase,
    trace: Vec<Phase>,
}

impl Default for PhaseMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl PhaseMachine {
    pub fn new() -> Self {
        Self {
            current: Phase::Init,
            trace: vec![Phase::Init],
        }
    }

    pub fn current(&self) -> Phase {
        self.current
    }

    pub fn trace(&self) -> &[Phase] {
        &self.trace
    }

    pub fn advance(&mut self, next: Phase) -> Result<()> {
        if !self.current.allows(next) {
            return Err(CoreError::Verification(format!(
                "Illegal phase transition {:?} -> {:?}",
                self.current, next
            )));
        }
        self.current = next;
        self.trace.push(next);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_legal_path_reaches_gate2_green() {
        let mut m = PhaseMachine::new();
        m.advance(Phase::Gate1Red).unwrap();
        m.advance(Phase::Repair).unwrap();
        m.advance(Phase::Gate2Green).unwrap();
        assert_eq!(
            m.trace(),
            &[
                Phase::Init,
                Phase::Gate1Red,
                Phase::Repair,
                Phase::Gate2Green
            ]
        );
        assert!(m.current().is_terminal());
    }

    #[test]
    fn test_cannot_skip_gate1() {
        let mut m = PhaseMachine::new();
        assert!(m.advance(Phase::Repair).is_err());
        assert!(m.advance(Phase::Gate2Green).is_err());
        assert_eq!(m.current(), Phase::Init);
    }

    #[test]
    fn test_cannot_reach_green_without_repair() {
        let mut m = PhaseMachine::new();
        m.advance(Phase::Gate1Red).unwrap();
        assert!(m.advance(Phase::Gate2Green).is_err());
    }

    #[test]
    fn test_terminal_states_are_final() {
        let mut m = PhaseMachine::new();
        m.advance(Phase::Rejected).unwrap();
        assert!(m.advance(Phase::Gate1Red).is_err());
        assert!(m.advance(Phase::Rejected).is_err());

        let mut g = PhaseMachine::new();
        g.advance(Phase::Gate1Red).unwrap();
        g.advance(Phase::Repair).unwrap();
        g.advance(Phase::Gate2Green).unwrap();
        assert!(g.advance(Phase::Rejected).is_err());
    }

    #[test]
    fn test_rejection_allowed_from_any_active_phase() {
        for steps in 0..3 {
            let mut m = PhaseMachine::new();
            let path = [Phase::Gate1Red, Phase::Repair];
            for p in path.iter().take(steps) {
                m.advance(*p).unwrap();
            }
            assert!(m.advance(Phase::Rejected).is_ok());
        }
    }
}
