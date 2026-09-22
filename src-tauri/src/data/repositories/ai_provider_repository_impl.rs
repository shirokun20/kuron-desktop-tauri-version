//! AiProviderRepositoryImpl — metadata di `KvStoreDs`, kunci di `SecretStore`
//! (keychain OS produksi / memori test). Kunci TIDAK PERNAH masuk file KV
//! atau payload IPC.

use async_trait::async_trait;

use crate::{
    core::AppError,
    data::datasources::local::{KvStoreDs, SecretStore},
    domain::{repositories::AiProviderRepository, AiProvider, AiProviderInput, AiProviderKind},
};

/// Metadata tersimpan tanpa `has_key` (dihitung ulang dari keychain).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct AiProviderMeta {
    id: String,
    name: String,
    kind: AiProviderKind,
    base_url: String,
    model: String,
}

impl AiProviderMeta {
    fn into_provider(self, has_key: bool) -> AiProvider {
        AiProvider {
            id: self.id,
            name: self.name,
            kind: self.kind,
            base_url: self.base_url,
            model: self.model,
            has_key,
        }
    }
}

pub struct AiProviderRepositoryImpl {
    kv: KvStoreDs,
    secret: SecretStore,
}

impl AiProviderRepositoryImpl {
    const KV_KEY: &'static str = "ai.providers";

    pub fn new(kv: KvStoreDs, secret: SecretStore) -> Self {
        Self { kv, secret }
    }

    /// Akun keychain per provider (service = APP_ID, lihat `SecretStore::new`).
    fn account(id: &str) -> String {
        format!("ai_provider.{id}")
    }

    fn load_meta(&self) -> Result<Vec<AiProviderMeta>, AppError> {
        match self.kv.get(Self::KV_KEY) {
            Some(raw) => serde_json::from_str(&raw)
                .map_err(|e| AppError::Storage(format!("metadata provider AI rusak: {e}"))),
            None => Ok(Vec::new()),
        }
    }

    fn save_meta(&self, list: &[AiProviderMeta]) -> Result<(), AppError> {
        let raw = serde_json::to_string(list)
            .map_err(|e| AppError::Storage(format!("serialisasi provider AI: {e}")))?;
        self.kv.set(Self::KV_KEY, &raw)
    }
}

#[async_trait]
impl AiProviderRepository for AiProviderRepositoryImpl {
    async fn list(&self) -> Result<Vec<AiProvider>, AppError> {
        let metas = self.load_meta()?;
        let mut out = Vec::with_capacity(metas.len());
        for m in metas {
            let has_key = self.secret.get_secret(&Self::account(&m.id))?.is_some();
            out.push(m.into_provider(has_key));
        }
        Ok(out)
    }

    async fn save(&self, input: AiProviderInput, api_key: &str) -> Result<AiProvider, AppError> {
        let id = input
            .id
            .clone()
            .ok_or_else(|| AppError::Validation("id provider wajib diisi".into()))?;
        let meta = AiProviderMeta {
            id: id.clone(),
            name: input.name.clone(),
            kind: input.kind,
            base_url: input.base_url.clone(),
            model: input.model.clone(),
        };
        // Kunci dulu ke keychain — bila gagal, metadata tak ditulis (jujur).
        self.secret.set_secret(&Self::account(&id), api_key)?;
        let mut list = self.load_meta()?;
        match list.iter().position(|m| m.id == id) {
            Some(i) => list[i] = meta.clone(),
            None => list.push(meta.clone()),
        }
        self.save_meta(&list)?;
        Ok(meta.into_provider(true))
    }

    async fn delete(&self, id: &str) -> Result<bool, AppError> {
        let mut list = self.load_meta()?;
        let Some(i) = list.iter().position(|m| m.id == id) else {
            return Ok(false);
        };
        list.remove(i);
        self.save_meta(&list)?;
        self.secret.delete_secret(&Self::account(id))?;
        Ok(true)
    }

    async fn get_key(&self, id: &str) -> Result<Option<String>, AppError> {
        self.secret.get_secret(&Self::account(id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::{
        DeleteAiProviderUseCase, ListAiProvidersUseCase, SaveAiProviderUseCase,
    };

    fn repo(dir: &std::path::Path) -> AiProviderRepositoryImpl {
        AiProviderRepositoryImpl::new(
            KvStoreDs::open(&dir.join("kv.json")).unwrap(),
            SecretStore::new_memory(),
        )
    }

    fn tmpdir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("kuron-ai-prov-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn input(id: Option<&str>, name: &str) -> AiProviderInput {
        AiProviderInput {
            id: id.map(String::from),
            name: name.into(),
            kind: AiProviderKind::OpenAi,
            base_url: String::new(),
            model: String::new(),
        }
    }

    #[test]
    fn save_list_delete_roundtrip_key_out_of_band() {
        let dir = tmpdir("round");
        let r = repo(&dir);
        let saved = tauri::async_runtime::block_on(
            SaveAiProviderUseCase::new(&r).execute(input(None, "My OpenAI"), "sk-test-123"),
        )
        .unwrap();
        assert_eq!(saved.id, "my-openai");
        assert!(saved.has_key);
        assert_eq!(saved.base_url, "https://api.openai.com/v1");
        assert_eq!(saved.model, "gpt-4o-mini");

        // Metadata di file KV; kunci TIDAK boleh ada di sana (plaintext off-band).
        let raw = std::fs::read_to_string(dir.join("kv.json")).unwrap();
        assert!(raw.contains("my-openai"), "metadata ada: {raw}");
        assert!(
            !raw.contains("sk-test-123"),
            "kunci bocor ke file KV: {raw}"
        );

        // Kunci terbaca backend saja (untuk translate 7.2), tak lewat list.
        let key = tauri::async_runtime::block_on(r.get_key("my-openai")).unwrap();
        assert_eq!(key.as_deref(), Some("sk-test-123"));

        let listed =
            tauri::async_runtime::block_on(ListAiProvidersUseCase::new(&r).execute()).unwrap();
        assert_eq!(listed.len(), 1);
        assert!(listed[0].has_key);
        // List tidak menyertakan field kunci sama sekali.
        let json = serde_json::to_string(&listed[0]).unwrap();
        assert!(!json.contains("sk-test-123"));

        let gone =
            tauri::async_runtime::block_on(DeleteAiProviderUseCase::new(&r).execute("my-openai"))
                .unwrap();
        assert!(gone);
        let after =
            tauri::async_runtime::block_on(ListAiProvidersUseCase::new(&r).execute()).unwrap();
        assert!(after.is_empty());
        let key_after = tauri::async_runtime::block_on(r.get_key("my-openai")).unwrap();
        assert!(key_after.is_none());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn save_rejects_empty_name_and_key() {
        let dir = tmpdir("valid");
        let r = repo(&dir);
        let err = tauri::async_runtime::block_on(
            SaveAiProviderUseCase::new(&r).execute(input(None, "   "), "sk-x"),
        )
        .unwrap_err();
        assert!(err.to_string().contains("nama"), "{err}");
        let err = tauri::async_runtime::block_on(
            SaveAiProviderUseCase::new(&r).execute(input(None, "Ok"), "  "),
        )
        .unwrap_err();
        assert!(err.to_string().contains("kunci"), "{err}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn save_upserts_same_id() {
        let dir = tmpdir("upsert");
        let r = repo(&dir);
        tauri::async_runtime::block_on(
            SaveAiProviderUseCase::new(&r).execute(input(Some("openai"), "OpenAI v1"), "sk-1"),
        )
        .unwrap();
        tauri::async_runtime::block_on(
            SaveAiProviderUseCase::new(&r).execute(input(Some("openai"), "OpenAI v2"), "sk-2"),
        )
        .unwrap();
        let listed =
            tauri::async_runtime::block_on(ListAiProvidersUseCase::new(&r).execute()).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "OpenAI v2");
        let key = tauri::async_runtime::block_on(r.get_key("openai")).unwrap();
        assert_eq!(key.as_deref(), Some("sk-2"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn delete_unknown_id_is_false() {
        let dir = tmpdir("miss");
        let r = repo(&dir);
        let gone =
            tauri::async_runtime::block_on(DeleteAiProviderUseCase::new(&r).execute("tidak-ada"))
                .unwrap();
        assert!(!gone);
        std::fs::remove_dir_all(&dir).ok();
    }
}
