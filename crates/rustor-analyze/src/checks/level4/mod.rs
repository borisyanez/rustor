//! Level 4 checks - Dead code detection

mod dead_code;
mod unused_result;
mod always_false_boolean;
mod write_only_property;

pub use dead_code::DeadCodeCheck;
pub use unused_result::UnusedResultCheck;
pub use always_false_boolean::AlwaysFalseBooleanCheck;
pub use write_only_property::WriteOnlyPropertyCheck;
