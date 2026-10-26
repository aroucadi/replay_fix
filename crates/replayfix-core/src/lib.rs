pub mod error;
pub mod platform;

pub use error::{CoreError, Result};
pub use platform::{OsFamily, Platform};
