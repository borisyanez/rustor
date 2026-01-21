//! Configuration handling for PHPStan-compatible analysis

pub mod composer;
pub mod neon;
pub mod phpstan;
pub mod level;

pub use phpstan::PhpStanConfig;
pub use level::Level;
