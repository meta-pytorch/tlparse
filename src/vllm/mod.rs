//! vLLM-specific parsing and visualization for tlparse.
//!
//! This module provides parsers and templates for vLLM's structured logs,
//! including piecewise compilation, subgraph tracking, and cudagraph captures.

pub mod parsers;
pub mod templates;
pub mod types;

pub use parsers::{generate_vllm_summary, vllm_parsers_with_state, VllmState};
pub use types::VllmSummaryContext;
