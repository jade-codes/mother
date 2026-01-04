//! Query module: Execute queries against Neo4j graph

mod run;

pub use run::run;

#[cfg(test)]
mod tests;
