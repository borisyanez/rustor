//! Level 4 checks - Dead code detection

mod dead_code;
mod unused_result;

pub use dead_code::DeadCodeCheck;
pub use unused_result::UnusedResultCheck;
