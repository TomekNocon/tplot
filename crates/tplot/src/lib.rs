//! Library facade for the `tplot` binary so integration tests can call into
//! its internals (e.g., the doctor formatter) without duplicating modules.
pub mod cli;
pub mod commands;
pub mod pipeline;
