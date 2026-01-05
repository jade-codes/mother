//! mother-cli library
//!
//! This module exposes the internal functionality of mother-cli for testing purposes.

use tracing_subscriber::{fmt, prelude::*, EnvFilter};

// Make commands module available for internal tests only
#[doc(hidden)]
pub mod commands;

pub mod types;
pub use types::QueryCommands;

/// Sets up the tracing subscriber for logging.
///
/// This function initializes the global tracing subscriber with a format layer
/// and an environment filter. The verbosity level determines the minimum log level.
///
/// # Arguments
///
/// * `verbose` - If `true`, sets the log level to "debug". If `false`, sets it to "info".
///
/// # Panics
///
/// This function will panic if the global subscriber has already been set.
///
/// # Examples
///
/// ```no_run
/// use mother_cli::setup_logging;
///
/// // Set up logging with info level
/// setup_logging(false);
///
/// // Set up logging with debug level
/// setup_logging(true);
/// ```
pub fn setup_logging(verbose: bool) {
    let filter = if verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(filter)
        .init();
}

#[cfg(test)]
mod tests;
