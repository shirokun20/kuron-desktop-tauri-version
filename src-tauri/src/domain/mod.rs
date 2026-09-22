//! Domain: pure — hanya `serde` (transport) + `ts-rs` (generate TS, ADR-004).
//! No Tauri/sqlite/reqwest imports. Port `lib/domain/entities/*` (Freezed).

pub mod entities;
pub mod repositories;
pub mod services;
#[cfg(test)]
mod ts_export;
pub mod value_objects;

pub use entities::{
    ai_provider::{AiProvider, AiProviderInput, AiProviderKind},
    ai_translation::{BubbleBox, PageTranslation},
    chapter::Chapter,
    comment::Comment,
    content::Content,
    download_task::{DownloadState, DownloadTask},
    glossary::GlossaryEntry,
    hello::Hello,
    history_item::HistoryItem,
    page_image_result::PageImageResult,
    reader_settings::{ReaderSettings, ReadingMode},
    search_filter::SearchFilter,
    tag::Tag,
};
pub use value_objects::{language::Language, source_id::SourceId};

#[cfg(test)]
mod roundtrip_tests {
    //! Roundtrip serde semua entity + value objects (3.1).
    //! Bandingkan via `serde_json::Value` agar tak perlu `PartialEq`.

    use serde::{de::DeserializeOwned, Serialize};

    use super::*;

    fn assert_roundtrip<T: Serialize + DeserializeOwned>(v: &T) {
        let json = serde_json::to_value(v).unwrap();
        let back: T = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(serde_json::to_value(&back).unwrap(), json);
    }

    #[test]
    fn hello_content_chapter() {
        assert_roundtrip(&Hello::new("Asix"));
        for c in Content::mock_feed() {
            assert_roundtrip(&c);
        }
        assert_roundtrip(&HistoryItem {
            content_id: "m1".into(),
            title: "Sample".into(),
            cover_url: "https://cdn.example/c.jpg".into(),
            source_id: "nhentai".into(),
            position: 9,
            updated_at: 1_700_000_000,
        });
        assert_roundtrip(&Tag {
            id: "33172".into(),
            name: "doujinshi".into(),
            tag_type: "category".into(),
            count: 508869,
        });
        assert_roundtrip(&Comment {
            id: "c9".into(),
            username: "anon".into(),
            body: "bagus!".into(),
            avatar_url: None,
            post_date: Some(1_700_000_000),
        });
        assert_roundtrip(&Chapter {
            id: "c1".into(),
            content_id: "m1".into(),
            title: "Chapter 1".into(),
            order: 1,
            is_external: true,
            external_url: Some("https://ex.example/1".into()),
            language: Some("en".into()),
        });
    }

    #[test]
    fn page_images_downloads() {
        assert_roundtrip(&PageImageResult::Cached("/tmp/p1.jpg".into()));
        assert_roundtrip(&PageImageResult::Remote(
            "https://cdn.example/p1.jpg".into(),
        ));
        for state in [
            DownloadState::Queued,
            DownloadState::Downloading { page: 3, total: 40 },
            DownloadState::Paused,
            DownloadState::Completed,
            DownloadState::Failed("timeout".into()),
        ] {
            assert_roundtrip(&state);
        }
        assert_roundtrip(&DownloadTask {
            chapter_id: "c1".into(),
            content_id: "m1".into(),
            state: DownloadState::Downloading { page: 3, total: 40 },
        });
    }

    #[test]
    fn ai_glossary_filter_reader() {
        assert_roundtrip(&PageTranslation {
            page_index: 0,
            bubbles: vec![BubbleBox {
                x: 1.0,
                y: 2.0,
                w: 3.0,
                h: 4.0,
                translated: Some("halo".into()),
            }],
        });
        assert_roundtrip(&GlossaryEntry {
            source: "senpai".into(),
            target: "kakak kelas".into(),
        });
        assert_roundtrip(&AiProviderKind::OpenAi);
        assert_roundtrip(&AiProvider {
            id: "openai".into(),
            name: "OpenAI".into(),
            kind: AiProviderKind::OpenAi,
            base_url: "https://api.openai.com/v1".into(),
            model: "gpt-4o-mini".into(),
            has_key: true,
        });
        assert_roundtrip(&AiProviderInput {
            id: None,
            name: "Kunci Pribadi".into(),
            kind: AiProviderKind::Custom,
            base_url: "https://llm.example/v1".into(),
            model: "llama-3".into(),
        });
        assert_roundtrip(&SearchFilter::default());
        assert_roundtrip(&SearchFilter {
            query: "test".into(),
            source_id: Some("nhentai".into()),
            page: 2,
        });
        for mode in [
            ReadingMode::Paginated,
            ReadingMode::ContinuousScroll,
            ReadingMode::Webtoon,
        ] {
            assert_roundtrip(&ReaderSettings {
                mode,
                right_to_left: true,
            });
        }
    }

    #[test]
    fn value_objects() {
        assert_roundtrip(&SourceId("nhentai".into()));
        assert_roundtrip(&Language::En);
        assert_roundtrip(&Language::Id);
        assert_roundtrip(&Language::Zh);
        assert_roundtrip(&Language::Other("ms".into()));
    }
}
