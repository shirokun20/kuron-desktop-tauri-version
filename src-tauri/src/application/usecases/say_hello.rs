//! SayHelloUseCase: port pattern `lib/domain/usecases/*` (spec §13).

use crate::{core::AppError, domain::Hello};

pub struct SayHelloUseCase;

impl SayHelloUseCase {
    pub fn new() -> Self {
        Self
    }

    pub fn execute(&self, name: Option<String>) -> Result<Hello, AppError> {
        Ok(Hello::new(name.unwrap_or_default()))
    }
}

impl Default for SayHelloUseCase {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_kuron_on_empty() {
        let hello = SayHelloUseCase::new().execute(None).unwrap();
        assert_eq!(hello.name, "Kuron");
        assert!(hello.message.contains("Kuron"));
    }

    #[test]
    fn uses_given_name() {
        let hello = SayHelloUseCase::new()
            .execute(Some("Asix".to_string()))
            .unwrap();
        assert_eq!(hello.name, "Asix");
        assert!(hello.message.contains("Asix"));
    }
}
