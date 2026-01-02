//! Version module: Scan run versioning
//!
//! Manages versioned scan runs, enabling diff queries
//! and change tracking between scans.

mod scan_run;

pub use scan_run::ScanRun;

#[cfg(test)]
mod tests;
