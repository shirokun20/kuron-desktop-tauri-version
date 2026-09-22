//! SaveAiProviderUseCase — validasi input + simpan provider BYOK (9.2).
//! Kunci non-kosong wajib; id kosong → slug dari nama.

use crate::{
    core::AppError,
    domain::{repositories::AiProviderRepository, AiProvider, AiProviderInput},
};

pub struct SaveAiProviderUseCase<R> {
    repo: R,
}

impl<R: AiProviderRepository> SaveAiProviderUseCase<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        input: AiProviderInput,
        api_key: &str,
    ) -> Result<AiProvider, AppError> {
        let name = input.name.trim();
        if name.is_empty() {
            return Err(AppError::Validation("nama provider wajib diisi".into()));
        }
        if api_key.trim().is_empty() {
            return Err(AppError::Validation("kunci API wajib diisi".into()));
        }
        let model = if input.model.trim().is_empty() {
            input.kind.default_model().to_string()
        } else {
            input.model.trim().to_string()
        };
        if model.is_empty() {
            return Err(AppError::Validation("model wajib diisi".into()));
        }
        let base_url = if input.base_url.trim().is_empty() {
            input.kind.default_base_url().to_string()
        } else {
            input.base_url.trim().to_string()
        };
        if base_url.is_empty() {
            return Err(AppError::Validation(
                "base URL wajib diisi untuk provider kustom".into(),
            ));
        }
        let id = match input.id.as_deref().map(str::trim) {
            Some(id) if !id.is_empty() => id.to_string(),
            _ => slugify(name),
        };
        if !valid_id(&id) {
            return Err(AppError::Validation(format!(
                "id provider '{id}' tak valid — tanpa slash/backslash atau titik"
            )));
        }
        self.repo
            .save(
                AiProviderInput {
                    id: Some(id),
                    name: name.to_string(),
                    kind: input.kind,
                    base_url,
                    model,
                },
                api_key.trim(),
            )
            .await
    }
}

/// Nama → id stabil: huruf kecil, non-alfanumerik jadi `-`, tanpa tepi `-`.
fn slugify(name: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for c in name.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
            prev_dash = false;
        } else if !prev_dash && !out.is_empty() {
            out.push('-');
            prev_dash = true;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "provider".to_string()
    } else {
        trimmed
    }
}

/// Id aman path/kunci keychain: kosong, `.`, `..`, separator, kontrol = tolak.
fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id != "."
        && id != ".."
        && !id.contains('/')
        && !id.contains('\\')
        && !id.chars().any(char::is_control)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify("My OpenAI Key"), "my-openai-key");
        assert_eq!(slugify("--Hi--There--"), "hi-there");
        assert_eq!(slugify("!!!"), "provider");
        assert_eq!(slugify("Kunci  Pribadi"), "kunci-pribadi");
    }

    #[test]
    fn valid_id_rejects_pathish() {
        assert!(valid_id("openai-1"));
        assert!(!valid_id(""));
        assert!(!valid_id("."));
        assert!(!valid_id(".."));
        assert!(!valid_id("a/b"));
        assert!(!valid_id("a\\b"));
    }
}
