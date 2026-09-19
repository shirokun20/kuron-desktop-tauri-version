//! Application: depends on Domain only (spec §9).
//! One use case = one file, one `execute` method.

pub mod usecases;

pub use usecases::{get_home_feed::GetHomeFeedUseCase, say_hello::SayHelloUseCase};
