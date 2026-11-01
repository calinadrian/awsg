//! Lightweight deterministic checks for the source-derived Elwynn selection and steering rules.
//! Run: rustc --edition 2024 --test scripts/elwynn_rules.rs -o out/elwynn-rules.exe

#[path = "../crates/benilla-app/src/survivors/elwynn_behavior.rs"]
mod elwynn_behavior;

#[allow(dead_code)]
#[path = "../crates/benilla-app/src/survivors/data.rs"]
mod data;
#[allow(dead_code)]
#[path = "../crates/benilla-app/src/survivors/elwynn.rs"]
mod elwynn;
