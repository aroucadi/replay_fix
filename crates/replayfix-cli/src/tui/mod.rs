//! Living Cybernetic Command Deck TUI components for ReplayFix Release 1.9.5.

pub mod brand;
pub mod dialog;
pub mod diff_view;
pub mod layout;
pub mod motion;
pub mod screen;
pub mod spinner;

pub use brand::{
    render_compact_hud_emblem, render_compact_hud_emblem_with_version, render_gradient_line,
    render_hero_splash, sample_brand_gradient,
};
pub use dialog::{prompt_selection, render_card};
pub use diff_view::{DiffReviewOutcome, DiffReviewer};
pub use layout::{
    BentoCommandDeck, Cell, CellStyle, DifferentialRenderer, FrameBuffer, ThoughtEvent,
    ThoughtGlyph,
};
pub use motion::{
    render_breathing_prompt, render_fractional_bar, render_wave_shimmer, GateFlashAnimation,
    GateFlashKind, MotionEngine,
};
pub use screen::ScreenGuard;
pub use spinner::StatusSpinner;
