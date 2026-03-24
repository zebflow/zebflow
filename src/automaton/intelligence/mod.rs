//! Automaton intelligence — AI-native capabilities beyond LLM/agentic reasoning.
//!
//! These are cognitive faculties that extend what an autonomous being can
//! perceive, process, and produce in the physical and digital world.
//! Each module represents a distinct form of machine intelligence,
//! decoupled from the agent reasoning layer.
//!
//! ## Planned capabilities
//!
//! - `tts/` — Text-to-Speech: synthesize spoken audio from text.
//! - `stt/` — Speech-to-Text: transcribe audio to text (Whisper-compatible).
//! - `ocr/` — Optical Character Recognition: extract text from images/documents.
//! - `vectorize/` — Embedding generation: encode text/images to vector space.
//! - `classify/` — Zero-shot or fine-tuned classification.
//! - `regression/` — Predictive modeling and numerical forecasting.
//!
//! ## Design principle
//!
//! Intelligence modules are STATELESS FUNCTIONS, not agents.
//! They take input, apply a model, and return output.
//! Agents in `crate::automaton::agents` CALL these as capabilities.
