//! rustor-rules: Refactoring rule implementations
//!
//! Available rules:
//! - array_push: Convert array_push($arr, $val) to $arr[] = $val
//! - is_null: Convert is_null($x) to $x === null
//! - sizeof: Convert sizeof($x) to count($x)

pub mod array_push;
pub mod is_null;
pub mod sizeof;

pub use array_push::check_array_push;
pub use is_null::check_is_null;
pub use sizeof::check_sizeof;
