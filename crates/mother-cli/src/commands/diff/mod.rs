//! Diff module: Compare commits or branches

mod run;

pub use run::run;

#[cfg(test)]
mod tests;
