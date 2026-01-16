//! Level 4 checks - Dead code detection

mod dead_code;
mod unused_result;
mod always_false_boolean;

pub use dead_code::DeadCodeCheck;
pub use unused_result::UnusedResultCheck;
pub use always_false_boolean::AlwaysFalseBooleanCheck;
