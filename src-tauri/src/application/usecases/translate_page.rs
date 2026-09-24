//! TranslatePageUseCase — orkestrasi translate satu halaman (7.2).
//! Port `ReaderTranslationCubit.translatePage` (tanpa state UI):
//! cache → provider default (fallback vision) → detect (prefetch/draw reuse)
//! → mosaic/full-image → AI → pasca (`_finish`: shape re-attach, flat flag,
//! font gaya, gagal kosong) → simpan cache. Round-robin 429 antar provider.
//! Satu use case = satu file + `execute` (aturan layer).

use sha2::{Digest, Sha256};

use crate::{
    core::AppError,
    data::{
        datasources::ai_translate::{self, MosaicBox},
        native::{
            bubble_detector::{post_process_boxes, BubbleDetector},
            image_ops,
        },
    },
    domain::{
        repositories::{AiProviderRepository, TranslationCacheRepository},
        AiProvider, BubbleBox, GlossaryEntry, PageTranslation, TranslationStyle,
    },
    network::HttpClientManager,
};

/// Versi skema cache — naikkan agar entri lama (tanpa shape) tak dipakai.
const CACHE_SCHEMA_VERSION: u32 = 2;

/// Entri glosarium relevan maks per prompt (port `glossaryMaxEntries`).
const GLOSSARY_MAX: usize = 5;

pub struct TranslatePageInput {
    /// Bytes halaman asli (JPEG/PNG/WebP).
    pub image: Vec<u8>,
    /// ID konten untuk kunci cache.
    pub content_id: String,
    /// Indeks halaman 0-based.
    pub page_index: u32,
    /// URL gambar (bagian kunci cache; beda crop = kunci beda).
    pub image_url: String,
    /// Offset Y viewport crop (crop beda = kunci cache beda).
    pub crop_y_top: u32,
    /// `true` = manga RTL, `false` = manhwa LTR (urutan baca prompt).
    pub rtl: bool,
    pub style: TranslationStyle,
    /// Gaya gagal (SFX saja) di-SKIP; false = terjemahkan semua.
    pub skip_sfx: bool,
    /// Bahasa target (mis. "id").
    pub target_lang: String,
    /// Bubble deteksi yang sudah ada (draw mode / prefetch) — reuse,
    /// tanpa deteksi ulang.
    pub pre_detected: Vec<BubbleBox>,
    /// Bubble manual user (pengganti otoritas): deteksi yang tertutup
    /// manual dibuang agar teks tak dikirim ganda ke AI.
    pub manual: Vec<BubbleBox>,
    /// Entri glosarium tersimpan (disaring relevan di sini, 9.3 baca nanti).
    pub glossary: Vec<GlossaryEntry>,
}

pub struct TranslatePageUseCase<A, C> {
    providers: A,
    cache: C,
    detector: std::sync::Arc<BubbleDetector>,
    http: HttpClientManager,
}

impl<A: AiProviderRepository, C: TranslationCacheRepository> TranslatePageUseCase<A, C> {
    pub fn new(
        providers: A,
        cache: C,
        detector: std::sync::Arc<BubbleDetector>,
        http: HttpClientManager,
    ) -> Self {
        Self {
            providers,
            cache,
            detector,
            http,
        }
    }

    /// Kunci cache: SHA256 16 hex ala cubit (`v{ver}:{content}:{page}:{urlHash}`).
    pub fn cache_key(
        content_id: &str,
        page_index: u32,
        image_url: &str,
        crop_y_top: u32,
    ) -> String {
        let mut h = Sha256::new();
        h.update(format!(
            "v{CACHE_SCHEMA_VERSION}:{content_id}:{page_index}:{}#{crop_y_top}",
            fx_hash(image_url)
        ));
        let hex: String = h.finalize().iter().map(|b| format!("{b:02x}")).collect();
        hex[..16].to_string()
    }

    pub async fn execute(&self, input: TranslatePageInput) -> Result<PageTranslation, AppError> {
        let key = Self::cache_key(
            &input.content_id,
            input.page_index,
            &input.image_url,
            input.crop_y_top,
        );
        // Manual = koreksi otoritas user → lewati cache (hasil lama tanpa manual).
        if input.manual.is_empty() {
            if let Some(cached) = self.cache.get(&key).await? {
                let shapes: Vec<Option<Vec<Vec<i32>>>> =
                    input.pre_detected.iter().map(|b| b.shape.clone()).collect();
                return Ok(attach_shapes(cached, &input.pre_detected, &shapes));
            }
        }

        let providers = self.providers.list().await?;
        let Some(active) = pick_provider(&providers) else {
            return Err(AppError::Validation(
                "belum ada provider AI — tambah dulu di Pengaturan → AI · Terjemahan".into(),
            ));
        };

        let (img_w, img_h) = image_dims(&input.image)?;
        let glossary_ctx = glossary_context(&input.glossary, &[]);
        let reading = if input.rtl {
            "right-to-left"
        } else {
            "left-to-right"
        };

        // Deteksi: reuse pre/manual, else ONNX (gagal = fallback full-image).
        let detected: Vec<BubbleBox> = if !input.pre_detected.is_empty() || !input.manual.is_empty()
        {
            input.pre_detected.clone()
        } else {
            match self.detector.detect(&input.image) {
                Ok(raw) => post_process_boxes(raw),
                Err(e) => {
                    tracing::warn!("deteksi bubble gagal, fallback full-image: {e}");
                    Vec::new()
                }
            }
        };
        // Manual menimpa: buang deteksi yang tertutup manual (>60% area kecil).
        let mut boxes: Vec<BubbleBox> = detected
            .iter()
            .filter(|d| !covered_by_manual(d, &input.manual))
            .cloned()
            .collect();
        boxes.extend(input.manual.iter().cloned());

        let (payload_jpeg, mosaic_boxes, shapes) = if boxes.is_empty() {
            // Fallback full-image: kompres halaman (decode+resize+JPEG85 CPU).
            let small = image_ops::compress_page(&input.image, 1600)
                .unwrap_or_else(|_| input.image.clone());
            (small, Vec::new(), Vec::new())
        } else {
            let rects: Vec<(u32, u32, u32, u32)> = boxes
                .iter()
                .map(|b| (b.x as u32, b.y as u32, b.w as u32, b.h as u32))
                .collect();
            let mosaic = image_ops::build_mosaic(&input.image, &rects)?;
            let mb: Vec<MosaicBox> = boxes
                .iter()
                .map(|b| MosaicBox {
                    x: b.x,
                    y: b.y,
                    w: b.w,
                    h: b.h,
                })
                .collect();
            let sh: Vec<Option<Vec<Vec<i32>>>> = boxes.iter().map(|b| b.shape.clone()).collect();
            (mosaic, mb, sh)
        };

        // Round-robin 429: tipe sama dulu, lalu tipe lain.
        let mut tried: Vec<String> = Vec::new();
        let mut current = active.clone();
        loop {
            let key_secret = self
                .providers
                .get_key(&current.id)
                .await?
                .filter(|k| !k.trim().is_empty())
                .ok_or_else(|| {
                    AppError::Validation(format!(
                        "provider '{}' belum punya kunci API",
                        current.name
                    ))
                })?;
            match ai_translate::translate_page(
                &self.http,
                &current,
                &key_secret,
                &payload_jpeg,
                img_w,
                img_h,
                &mosaic_boxes,
                &shapes,
                &input.target_lang,
                input.style,
                input.skip_sfx,
                reading,
                glossary_ctx.as_deref(),
            )
            .await
            {
                Ok(mut result) => {
                    result.page_index = input.page_index;
                    let all_shapes: Vec<Option<Vec<Vec<i32>>>> =
                        detected.iter().map(|b| b.shape.clone()).collect();
                    let mut finished = attach_shapes(result, &detected, &all_shapes);
                    finished = flag_flat_bubbles(finished, img_w, img_h);
                    self.cache.put(&key, &finished).await?;
                    return Ok(finished);
                }
                Err(AppError::RateLimited(_)) => {
                    tried.push(current.id.clone());
                    if let Some(next) = fallback_provider(&current, &providers, &tried) {
                        tracing::warn!(
                            "provider '{}' rate-limited → '{}'",
                            current.name,
                            next.name
                        );
                        current = next;
                        continue;
                    }
                    return Err(AppError::RateLimited(
                        "semua provider AI rate-limited (429) — tunggu ±60 dtk".into(),
                    ));
                }
                Err(e) => return Err(e),
            }
        }
    }
}

fn fx_hash(s: &str) -> u64 {
    // FNV-1a 64 (cukup untuk bagian kunci; SHA luar yang final).
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn image_dims(data: &[u8]) -> Result<(u32, u32), AppError> {
    let img =
        image::load_from_memory(data).map_err(|e| AppError::Internal(format!("decode: {e}")))?;
    Ok((img.width(), img.height()))
}

/// Provider aktif: default dulu, else pertama; teks-saja digeser ke vision
/// bila ada (port logika cubit; flag vision = heuristik nama model).
fn pick_provider(providers: &[AiProvider]) -> Option<AiProvider> {
    if providers.is_empty() {
        return None;
    }
    // Desktop belum simpan flag `is_default` (9.2 tanpa default) → pertama.
    let first = providers[0].clone();
    if is_vision_capable(&first) {
        return Some(first);
    }
    if let Some(v) = providers.iter().find(|p| is_vision_capable(p)) {
        return Some(v.clone());
    }
    Some(first)
}

fn is_vision_capable(p: &AiProvider) -> bool {
    let m = p.model.to_lowercase();
    // Heuristik nama (tanpa flag LOV tersimpan): tolak yang jelas teks-saja.
    !(m.contains("embedding")
        || m.contains("whisper")
        || m.contains("tts")
        || m.contains("transcribe"))
}

fn fallback_provider(
    current: &AiProvider,
    providers: &[AiProvider],
    tried: &[String],
) -> Option<AiProvider> {
    let same = providers
        .iter()
        .filter(|p| p.id != current.id && !tried.contains(&p.id))
        .find(|p| std::mem::discriminant(&p.kind) == std::mem::discriminant(&current.kind));
    if let Some(p) = same {
        return Some(p.clone());
    }
    providers.iter().find(|p| !tried.contains(&p.id)).cloned()
}

/// Glosarium relevan: source muncul di teks (case-insensitive), terbaru
/// dulu, maks 5 (port `selectRelevantGlossaryEntries`).
pub fn select_glossary(entries: &[GlossaryEntry], texts: &[String]) -> Vec<GlossaryEntry> {
    if entries.is_empty() {
        return Vec::new();
    }
    let hay: Vec<String> = texts
        .iter()
        .map(|t| t.trim().to_lowercase())
        .filter(|t| !t.is_empty())
        .collect();
    let mut matched: Vec<GlossaryEntry> = entries
        .iter()
        .filter(|e| {
            let needle = e.source.trim().to_lowercase();
            if needle.is_empty() {
                return false;
            }
            if hay.is_empty() {
                return true;
            }
            hay.iter().any(|h| h.contains(&needle))
        })
        .cloned()
        .collect();
    matched.truncate(GLOSSARY_MAX);
    matched
}

/// Blok `Glossary:\n"src" -> "dst"` — disuntik ke prompt yang SAMA.
pub fn glossary_context(entries: &[GlossaryEntry], texts: &[String]) -> Option<String> {
    let rel = select_glossary(entries, texts);
    if rel.is_empty() {
        return None;
    }
    let lines: Vec<String> = rel
        .iter()
        .map(|e| format!("\"{}\" -> \"{}\"", e.source, e.target))
        .collect();
    Some(format!("Glossary:\n{}", lines.join("\n")))
}

/// Tempel ulang poligon ONNX ke hasil AI (bentuk tak selamat via mosaic).
/// Manual rect tanpa shape tak pernah diberi poligon deteksi.
fn attach_shapes(
    result: PageTranslation,
    detected: &[BubbleBox],
    _shapes: &[Option<Vec<Vec<i32>>>],
) -> PageTranslation {
    if detected.is_empty() {
        return result;
    }
    let bubbles = result
        .bubbles
        .into_iter()
        .map(|mut b| {
            if b.shape.is_some() {
                return b;
            }
            b.shape = nearest_shape(detected, b.x, b.y, b.w, b.h);
            b
        })
        .collect();
    PageTranslation { bubbles, ..result }
}

fn nearest_shape(detected: &[BubbleBox], x: f32, y: f32, w: f32, h: f32) -> Option<Vec<Vec<i32>>> {
    let (mut best, mut best_score) = (None, f32::NEG_INFINITY);
    for d in detected {
        let shape = d.shape.clone().filter(|s| s.len() >= 3)?;
        let iou = box_iou((d.x, d.y, d.w, d.h), (x, y, w, h));
        let dx = ((d.x + d.w / 2.0) - (x + w / 2.0)).abs();
        let dy = ((d.y + d.h / 2.0) - (y + h / 2.0)).abs();
        let score = iou - (dx + dy) * 1e-4;
        if score > best_score {
            best_score = score;
            best = Some(shape);
        }
    }
    best
}

fn box_iou(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> f32 {
    let ix1 = a.0.max(b.0);
    let iy1 = a.1.max(b.1);
    let ix2 = (a.0 + a.2).min(b.0 + b.2);
    let iy2 = (a.1 + a.3).min(b.1 + b.3);
    let (iw, ih) = ((ix2 - ix1).max(0.0), (iy2 - iy1).max(0.0));
    let inter = iw * ih;
    if inter <= 0.0 {
        return 0.0;
    }
    let ua = a.2 * a.3 + b.2 * b.3 - inter;
    if ua <= 0.0 {
        0.0
    } else {
        inter / ua
    }
}

/// True bila manual menutupi >60% area terkecil (duplikat → deteksi dibuang).
fn covered_by_manual(d: &BubbleBox, manual: &[BubbleBox]) -> bool {
    let area_d = d.w * d.h;
    if area_d <= 0.0 {
        return false;
    }
    manual.iter().any(|m| {
        let ix1 = d.x.max(m.x);
        let iy1 = d.y.max(m.y);
        let ix2 = (d.x + d.w).min(m.x + m.w);
        let iy2 = (d.y + d.h).min(m.y + m.h);
        let inter = ((ix2 - ix1).max(0.0)) * ((iy2 - iy1).max(0.0));
        if inter <= 0.0 {
            return false;
        }
        let smaller = area_d.min(m.w * m.h);
        smaller > 0.0 && inter / smaller >= 0.6
    })
}

/// Heuristik "bubble flat" cypy: rasio ≥2.4 + lebar ≥45% + tinggi ≤22% →
/// teks di atas artwork ramai, butuh patch putih (port `_flagFlatBubbles`).
fn flag_flat_bubbles(mut result: PageTranslation, img_w: u32, img_h: u32) -> PageTranslation {
    let (w, h) = (img_w as f32, img_h as f32);
    for b in &mut result.bubbles {
        b.needs_white_patch = b.w / b.h.max(1.0) >= 2.4 && b.w >= w * 0.45 && b.h <= h * 0.22;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::TranslatedBubble;

    fn bubble(x: f32, y: f32, w: f32, h: f32, kind: &str) -> BubbleBox {
        BubbleBox {
            x,
            y,
            w,
            h,
            confidence: 0.9,
            shape: None,
            kind: Some(kind.into()),
            tail: None,
            translated: None,
        }
    }

    #[test]
    fn kunci_cache_stabil_dan_beda_crop() {
        let a = TranslatePageUseCase::<MockP, MockC>::cache_key("c1", 2, "u", 0);
        let b = TranslatePageUseCase::<MockP, MockC>::cache_key("c1", 2, "u", 0);
        let c = TranslatePageUseCase::<MockP, MockC>::cache_key("c1", 2, "u", 99);
        assert_eq!(a.len(), 16);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    // Mock minimal untuk test kunci (tanpa network).
    struct MockP;
    struct MockC;
    #[async_trait::async_trait]
    impl AiProviderRepository for MockP {
        async fn list(&self) -> Result<Vec<AiProvider>, AppError> {
            Ok(vec![])
        }
        async fn save(
            &self,
            _i: crate::domain::AiProviderInput,
            _k: &str,
        ) -> Result<AiProvider, AppError> {
            unreachable!()
        }
        async fn delete(&self, _id: &str) -> Result<bool, AppError> {
            Ok(false)
        }
        async fn get_key(&self, _id: &str) -> Result<Option<String>, AppError> {
            Ok(None)
        }
    }
    #[async_trait::async_trait]
    impl TranslationCacheRepository for MockC {
        async fn get(&self, _k: &str) -> Result<Option<PageTranslation>, AppError> {
            Ok(None)
        }
        async fn put(&self, _k: &str, _v: &PageTranslation) -> Result<(), AppError> {
            Ok(())
        }
    }

    #[test]
    fn teks_dalam_balloon_dibuang_post_process() {
        let boxes = vec![
            bubble(0.0, 0.0, 200.0, 100.0, "balloon"),
            bubble(20.0, 20.0, 50.0, 30.0, "text"),
            bubble(300.0, 300.0, 60.0, 40.0, "text"),
        ];
        let out = post_process_boxes(boxes);
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn manual_menutupi_deteksi() {
        let d = bubble(0.0, 0.0, 100.0, 100.0, "balloon");
        let m = bubble(10.0, 10.0, 80.0, 80.0, "balloon");
        assert!(covered_by_manual(&d, &[m]));
        let jauh = bubble(500.0, 500.0, 10.0, 10.0, "balloon");
        assert!(!covered_by_manual(&d, &[jauh]));
    }

    #[test]
    fn glossary_disaring_relevan() {
        let entries = vec![
            GlossaryEntry {
                source: "senpai".into(),
                target: "kakak".into(),
            },
            GlossaryEntry {
                source: "zzz".into(),
                target: "tidur".into(),
            },
        ];
        let rel = select_glossary(&entries, &["Halo senpai!".to_string()]);
        assert_eq!(rel.len(), 1);
        assert_eq!(rel[0].target, "kakak");
        let ctx = glossary_context(&entries, &["Halo senpai!".to_string()]).unwrap();
        assert!(ctx.contains("\"senpai\" -> \"kakak\""));
    }

    #[test]
    fn flat_bubble_ditandai() {
        let r = PageTranslation {
            page_index: 0,
            bubbles: vec![TranslatedBubble {
                x: 0.0,
                y: 0.0,
                w: 900.0,
                h: 100.0,
                original: String::new(),
                reading: String::new(),
                translated: "HAH".into(),
                shape: None,
                needs_white_patch: false,
            }],
            detected_lang: String::new(),
            used_fallback: false,
        };
        let out = flag_flat_bubbles(r, 1000, 1500);
        assert!(out.bubbles[0].needs_white_patch);
    }
}
