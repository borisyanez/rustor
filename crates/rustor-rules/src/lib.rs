//! rustor-rules: Refactoring rule implementations
//!
//! Available rules:
//! - array_push: Convert array_push($arr, $val) to $arr[] = $val
//! - empty_coalesce: Convert empty($x) ? $default : $x to $x ?: $default
//! - is_null: Convert is_null($x) to $x === null
//! - isset_coalesce: Convert isset($x) ? $x : $default to $x ?? $default
//! - sizeof: Convert sizeof($x) to count($x)
//! - type_cast: Convert strval/intval/floatval to cast syntax

pub mod array_push;
pub mod empty_coalesce;
pub mod is_null;
pub mod isset_coalesce;
pub mod sizeof;
pub mod type_cast;

pub use array_push::check_array_push;
pub use empty_coalesce::check_empty_coalesce;
pub use is_null::check_is_null;
pub use isset_coalesce::check_isset_coalesce;
pub use sizeof::check_sizeof;
pub use type_cast::check_type_cast;
