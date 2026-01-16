//! Level 4 checks - Dead code detection

mod dead_code;
mod unused_result;
mod always_false_boolean;
mod write_only_property;
mod invalid_binary_op;

pub use dead_code::DeadCodeCheck;
pub use unused_result::UnusedResultCheck;
pub use always_false_boolean::AlwaysFalseBooleanCheck;
pub use write_only_property::WriteOnlyPropertyCheck;
pub use invalid_binary_op::InvalidBinaryOpCheck;
