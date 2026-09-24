//! bubble_detector — port `BubbleDetector.kt` (kuron-mobile `kuron_native`, 5.2).
//! YOLO26s-seg 1280×1280: letterbox → `ort` CPU → box [1,300,38] + proto
//! [1,32,320,320] → mask → poligon → NMS IoU 0.45 per kelas.
//! Kelas: 0=frame, 1=text/thought, 2=balloon (semua diikutkan ala mobile,
//! `kind` dibedakan; NMS hanya menekan kelas yang sama).
//! Model 42MB diunduh sekali dari repo mobile ke data-dir (keputusan user),
//! bukan dibundel. Tanpa model = error jelas, bukan silent kosong.

// Pipeline inferensi hanya terjangkau saat fitur `ort` aktif; tanpa itu
// helper-nya memang mati — bungkam dead_code presisi (bukan global).
#![cfg_attr(not(feature = "ort"), allow(dead_code))]

use std::path::{Path, PathBuf};
#[cfg(feature = "ort")]
use std::sync::Mutex;

use image::{DynamicImage, GenericImageView};

use crate::core::AppError;
use crate::domain::BubbleBox;

pub const MODEL_INPUT: u32 = 1280;
pub const PROTO_SIZE: u32 = 320;
pub const NUM_DET: usize = 300;
pub const ROW_LEN: usize = 38;
pub const NMS_IOU: f32 = 0.45;

/// Ambang conf ala mobile: narasi (cls 1) sering 0.15–0.24, balloon ketat.
const CONF_TEXT: f32 = 0.15;
const CONF_OTHER: f32 = 0.25;
const MASK_THRESHOLD: f32 = 0.5;
const POLY_EPSILON_FRAC: f32 = 0.015;
const RENDER_PAD: f32 = 3.0;
const MIN_BOX_DIM: f32 = 4.0;

/// URL model di repo mobile (aset `BubbleDetector.kt`).
pub const BUBBLE_MODEL_URL: &str = "https://raw.githubusercontent.com/shirokun20/kuron-mobile/master/packages/kuron_native/android/src/main/assets/bubble_detector.onnx";

fn err(msg: String) -> AppError {
    AppError::Internal(msg)
}

/// Satu kandidat mentah (koordinat letterbox) sebelum NMS.
#[derive(Debug, Clone)]
struct RawDet {
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    conf: f32,
    cls: i32,
    coeff: [f32; 32],
}

fn iou(a: &RawDet, b: &RawDet) -> f32 {
    let ix1 = a.x1.max(b.x1);
    let iy1 = a.y1.max(b.y1);
    let ix2 = a.x2.min(b.x2);
    let iy2 = a.y2.min(b.y2);
    let iw = (ix2 - ix1).max(0.0);
    let ih = (iy2 - iy1).max(0.0);
    let inter = iw * ih;
    if inter <= 0.0 {
        return 0.0;
    }
    let ua = (a.x2 - a.x1) * (a.y2 - a.y1) + (b.x2 - b.x1) * (b.y2 - b.y1) - inter;
    if ua <= 0.0 {
        0.0
    } else {
        inter / ua
    }
}

/// NMS per kelas: urut conf desc, tekan IoU > `NMS_IOU` (0.45) kelas sama.
fn nms(mut dets: Vec<RawDet>) -> Vec<RawDet> {
    dets.sort_by(|a, b| {
        b.conf
            .partial_cmp(&a.conf)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut kept: Vec<RawDet> = Vec::with_capacity(dets.len());
    'outer: for d in dets {
        for k in &kept {
            if d.cls == k.cls && iou(&d, k) > NMS_IOU {
                continue 'outer;
            }
        }
        kept.push(d);
    }
    kept
}

fn kind_of(cls: i32) -> &'static str {
    match cls {
        2 => "balloon",
        1 => "text",
        0 => "frame",
        _ => "unknown",
    }
}

/// Post-processing deteksi mentah ala cubit mobile (`postProcessBoxes`):
/// teks yang terserap balloon dibuang, frame dibuang, NMS 0.45, lalu
/// false-positive raksasa (menelan box >2.5× lebih kecil) dibuang.
pub fn post_process_boxes(boxes: Vec<BubbleBox>) -> Vec<BubbleBox> {
    if boxes.is_empty() {
        return Vec::new();
    }
    let mut sorted = boxes;
    sorted.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let balloons: Vec<(f32, f32, f32, f32)> = sorted
        .iter()
        .filter(|b| b.kind.as_deref() == Some("balloon"))
        .map(|b| (b.x, b.y, b.w, b.h))
        .collect();
    sorted.retain(|b| {
        if b.kind.as_deref() == Some("frame") {
            return false;
        }
        if b.kind.as_deref() == Some("text") {
            let (x, y, w, h) = (b.x, b.y, b.w, b.h);
            if balloons
                .iter()
                .any(|&(ox, oy, ow, oh)| ox <= x && oy <= y && ox + ow >= x + w && oy + oh >= y + h)
            {
                return false;
            }
        }
        true
    });
    let mut kept: Vec<BubbleBox> = Vec::with_capacity(sorted.len());
    for b in sorted {
        let dominated = kept
            .iter()
            .any(|k: &BubbleBox| box_iou((k.x, k.y, k.w, k.h), (b.x, b.y, b.w, b.h)) > NMS_IOU);
        if !dominated {
            kept.push(b);
        }
    }
    // Aturan cypy PR#2: raksasa yang menelan box >2.5× lebih kecil = salah.
    let mut drop = vec![false; kept.len()];
    for (i, a) in kept.iter().enumerate() {
        for (j, b) in kept.iter().enumerate() {
            if i == j {
                continue;
            }
            let area_a = a.w * a.h;
            let area_b = b.w * b.h;
            if area_b * 2.5 < area_a
                && a.x <= b.x
                && a.y <= b.y
                && a.x + a.w >= b.x + b.w
                && a.y + a.h >= b.y + b.h
            {
                drop[i] = true;
            }
        }
    }
    kept.into_iter()
        .enumerate()
        .filter(|(i, _)| !drop[*i])
        .map(|(_, b)| b)
        .collect()
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

/// Letterbox: skala + padding agar 1280×1280 tanpa stretch (ala mobile).
fn letterbox(img: &DynamicImage) -> (Vec<f32>, f32, f32, f32, u32, u32) {
    let (w, h) = img.dimensions();
    let scale = (MODEL_INPUT as f32 / w as f32).min(MODEL_INPUT as f32 / h as f32);
    let nw = ((w as f32 * scale).round() as u32).max(1);
    let nh = ((h as f32 * scale).round() as u32).max(1);
    let pad_l = (MODEL_INPUT - nw) as f32 / 2.0;
    let pad_t = (MODEL_INPUT - nh) as f32 / 2.0;
    let small = img.resize_exact(nw, nh, image::imageops::FilterType::Triangle);
    let rgb = small.to_rgb8();
    // Kanvas hitam 1280² (mobile: bitmap transparan → RGB 0,0,0).
    let mut chw = vec![0.0f32; 3 * MODEL_INPUT as usize * MODEL_INPUT as usize];
    let stride = MODEL_INPUT as usize * MODEL_INPUT as usize;
    let ox = pad_l.floor() as u32;
    let oy = pad_t.floor() as u32;
    for y in 0..nh {
        for x in 0..nw {
            let p = rgb.get_pixel(x, y);
            let dx = (ox + x) as usize;
            let dy = (oy + y) as usize;
            chw[dy * MODEL_INPUT as usize + dx] = p[0] as f32 / 255.0;
            chw[stride + dy * MODEL_INPUT as usize + dx] = p[1] as f32 / 255.0;
            chw[2 * stride + dy * MODEL_INPUT as usize + dx] = p[2] as f32 / 255.0;
        }
    }
    (chw, scale, pad_l, pad_t, w, h)
}

fn sigmoid(v: f32) -> f32 {
    1.0 / (1.0 + (-v.clamp(-80.0, 80.0)).exp())
}

/// Bilinear resize ala `cv2.INTER_LINEAR` (port `resizeBilinear`).
fn resize_bilinear(src: &[f32], sw: usize, sh: usize, dw: usize, dh: usize) -> Vec<f32> {
    let mut dst = vec![0.0f32; dw * dh];
    let xr = if dw > 1 { sw as f64 / dw as f64 } else { 0.0 };
    let yr = if dh > 1 { sh as f64 / dh as f64 } else { 0.0 };
    for y in 0..dh {
        let sy = y as f64 * yr;
        let y0 = (sy as usize).min(sh - 1);
        let fy = (sy - y0 as f64).clamp(0.0, 1.0) as f32;
        let y1 = (y0 + 1).min(sh - 1);
        for x in 0..dw {
            let sx = x as f64 * xr;
            let x0 = (sx as usize).min(sw - 1);
            let fx = (sx - x0 as f64).clamp(0.0, 1.0) as f32;
            let x1 = (x0 + 1).min(sw - 1);
            let top = src[y0 * sw + x0] + (src[y0 * sw + x1] - src[y0 * sw + x0]) * fx;
            let bot = src[y1 * sw + x0] + (src[y1 * sw + x1] - src[y1 * sw + x0]) * fx;
            dst[y * dw + x] = top + (bot - top) * fy;
        }
    }
    dst
}

/// coeff(32) · proto → sigmoid → crop ke box (proto-space) → resize → biner.
fn decode_mask(
    coeff: &[f32; 32],
    proto: &[f32],
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
) -> Option<(Vec<bool>, usize, usize)> {
    let s = PROTO_SIZE as f32 / MODEL_INPUT as f32;
    let ps = PROTO_SIZE as usize;
    let px1 = ((x1 * s) as usize).min(ps - 1);
    let py1 = ((y1 * s) as usize).min(ps - 1);
    let px2 = ((x2 * s) as usize).min(ps);
    let py2 = ((y2 * s) as usize).min(ps);
    if px2 <= px1 || py2 <= py1 {
        return None;
    }
    let (cw, ch) = (px2 - px1, py2 - py1);
    let mut prob = vec![0.0f32; cw * ch];
    for y in 0..ch {
        for x in 0..cw {
            let mut v = 0.0f32;
            let idx = (py1 + y) * ps + (px1 + x);
            for c in 0..32 {
                v += coeff[c] * proto[c * ps * ps + idx];
            }
            prob[y * cw + x] = sigmoid(v);
        }
    }
    let bw = ((x2 - x1) as usize).max(1);
    let bh = ((y2 - y1) as usize).max(1);
    let resized = resize_bilinear(&prob, cw, ch, bw, bh);
    Some((
        resized.iter().map(|&v| v >= MASK_THRESHOLD).collect(),
        bw,
        bh,
    ))
}

fn is_edge(mask: &[bool], w: usize, h: usize, x: usize, y: usize) -> bool {
    x == 0
        || y == 0
        || x == w - 1
        || y == h - 1
        || !mask[y * w + (x - 1)]
        || !mask[y * w + (x + 1)]
        || !mask[(y - 1) * w + x]
        || !mask[(y + 1) * w + x]
}

/// Moore-neighbor tracing kontur luar (port `traceOuterContour`).
fn trace_outer_contour(mask: &[bool], w: usize, h: usize) -> Option<Vec<[i32; 2]>> {
    let mut start: Option<[i32; 2]> = None;
    for y in 0..h {
        for x in 0..w {
            if mask[y * w + x] && is_edge(mask, w, h, x, y) {
                start = Some([x as i32, y as i32]);
                break;
            }
        }
        if start.is_some() {
            break;
        }
    }
    let s = start?;
    const DX: [i32; 8] = [1, 1, 0, -1, -1, -1, 0, 1];
    const DY: [i32; 8] = [0, 1, 1, 1, 0, -1, -1, -1];
    let mut path = vec![s];
    let mut cur = s;
    let mut dir = 7usize;
    let max_iter = w * h * 4;
    let mut guard = 0usize;
    loop {
        guard += 1;
        if guard > max_iter {
            break;
        }
        let mut found = false;
        for k in 0..8 {
            let nd = (dir + 1 + k) % 8;
            let nx = cur[0] + DX[nd];
            let ny = cur[1] + DY[nd];
            if nx >= 0 && ny >= 0 && (nx as usize) < w && (ny as usize) < h {
                let (ux, uy) = (nx as usize, ny as usize);
                if mask[uy * w + ux] && is_edge(mask, w, h, ux, uy) {
                    cur = [nx, ny];
                    path.push(cur);
                    dir = (nd + 4) % 8;
                    found = true;
                    break;
                }
            }
        }
        if !found || cur == s {
            break;
        }
    }
    if path.len() >= 2 && path.first() == path.last() {
        path.pop();
    }
    if path.len() < 3 {
        None
    } else {
        Some(path)
    }
}

fn dist(a: &[i32; 2], b: &[i32; 2]) -> f32 {
    (((a[0] - b[0]).pow(2) + (a[1] - b[1]).pow(2)) as f32).sqrt()
}

fn perp_dist(p: &[i32; 2], a: &[i32; 2], b: &[i32; 2]) -> f32 {
    let len = dist(a, b);
    if len < 1e-6 {
        return dist(p, a);
    }
    let cross = ((b[0] - a[0]) * (a[1] - p[1]) - (a[0] - p[0]) * (b[1] - a[1])).abs() as f32;
    cross / len
}

fn rdp(ring: &[[i32; 2]], first: usize, last: usize, keep: &mut [bool], eps: f32) {
    if last <= first + 1 {
        return;
    }
    let (mut max_d, mut idx) = (0.0f32, first);
    for i in (first + 1)..last {
        let d = perp_dist(&ring[i], &ring[first], &ring[last]);
        if d > max_d {
            max_d = d;
            idx = i;
        }
    }
    if max_d > eps {
        keep[idx] = true;
        rdp(ring, first, idx, keep, eps);
        rdp(ring, idx, last, keep, eps);
    }
}

/// Ramer–Douglas–Peucker ring tertutup (port `approxPolyDP`).
fn approx_poly_dp(contour: &[[i32; 2]]) -> Vec<[i32; 2]> {
    let mut ring: Vec<[i32; 2]> = contour.to_vec();
    ring.push(contour[0]);
    let mut perim = 0.0f32;
    for i in 0..ring.len() - 1 {
        perim += dist(&ring[i], &ring[i + 1]);
    }
    let eps = POLY_EPSILON_FRAC * perim;
    let mut keep = vec![false; ring.len()];
    keep[0] = true;
    keep[ring.len() - 1] = true;
    rdp(&ring, 0, ring.len() - 1, &mut keep, eps);
    let mut out: Vec<[i32; 2]> = ring
        .iter()
        .enumerate()
        .filter(|(i, _)| keep[*i])
        .map(|(_, &p)| p)
        .collect();
    if out.len() >= 2 && out.first() == out.last() {
        out.pop();
    }
    out
}

/// Mask biner → poligon ruang letterbox (offset ke box). None = pakai box.
fn mask_to_polygon_lb(
    mask: &[bool],
    w: usize,
    h: usize,
    bx1: f32,
    by1: f32,
) -> Option<Vec<[f32; 2]>> {
    let contour = trace_outer_contour(mask, w, h)?;
    if contour.len() < 3 {
        return None;
    }
    let simp = approx_poly_dp(&contour);
    if simp.len() < 3 {
        return None;
    }
    Some(
        simp.iter()
            .map(|p| [bx1 + p[0] as f32, by1 + p[1] as f32])
            .collect(),
    )
}

/// Letterbox → koordinat asli, dijepit.
fn lb_to_orig(
    pts: &[[f32; 2]],
    ow: u32,
    oh: u32,
    scale: f32,
    pad_l: f32,
    pad_t: f32,
) -> Vec<[i32; 2]> {
    pts.iter()
        .map(|[x, y]| {
            [
                (((x - pad_l) / scale) as i32).clamp(0, ow as i32 - 1),
                (((y - pad_t) / scale) as i32).clamp(0, oh as i32 - 1),
            ]
        })
        .collect()
}

/// Kembangkan poligon ke luar sejauh `RENDER_PAD` px (port `expandPolygon`).
fn expand_polygon(pts: &[[i32; 2]], ow: u32, oh: u32) -> Vec<Vec<i32>> {
    let n = pts.len() as f32;
    let (mut cx, mut cy) = (0.0f32, 0.0f32);
    for p in pts {
        cx += p[0] as f32;
        cy += p[1] as f32;
    }
    cx /= n;
    cy /= n;
    pts.iter()
        .map(|p| {
            let (dx, dy) = (p[0] as f32 - cx, p[1] as f32 - cy);
            let norm = (dx * dx + dy * dy).sqrt();
            let (nx, ny) = if norm < 1e-6 {
                (0.0, 0.0)
            } else {
                (dx / norm * RENDER_PAD, dy / norm * RENDER_PAD)
            };
            vec![
                ((p[0] as f32 + nx) as i32).clamp(0, ow as i32 - 1),
                ((p[1] as f32 + ny) as i32).clamp(0, oh as i32 - 1),
            ]
        })
        .collect()
}

/// Bangun `BubbleBox` dari kandidat + proto (box selalu ada, shape opsional).
fn to_bubble(
    d: &RawDet,
    proto: &[f32],
    ow: u32,
    oh: u32,
    scale: f32,
    pad_l: f32,
    pad_t: f32,
) -> BubbleBox {
    let bx1 = (((d.x1 - pad_l) / scale) as i32).clamp(0, ow as i32 - 1);
    let by1 = (((d.y1 - pad_t) / scale) as i32).clamp(0, oh as i32 - 1);
    let bx2 = (((d.x2 - pad_l) / scale) as i32).clamp(0, ow as i32 - 1);
    let by2 = (((d.y2 - pad_t) / scale) as i32).clamp(0, oh as i32 - 1);
    let shape = decode_mask(&d.coeff, proto, d.x1, d.y1, d.x2, d.y2).and_then(|(bin, bw, bh)| {
        let poly = mask_to_polygon_lb(&bin, bw, bh, d.x1, d.y1)?;
        if poly.len() < 3 {
            return None;
        }
        let orig = lb_to_orig(&poly, ow, oh, scale, pad_l, pad_t);
        Some(expand_polygon(&orig, ow, oh))
    });
    BubbleBox {
        x: bx1 as f32,
        y: by1 as f32,
        w: (bx2 - bx1).max(1) as f32,
        h: (by2 - by1).max(1) as f32,
        confidence: d.conf as f64,
        shape,
        kind: Some(kind_of(d.cls).to_string()),
        tail: None,
        translated: None,
    }
}

fn extract_candidates(det: &[f32]) -> Vec<RawDet> {
    let mut out = Vec::new();
    for i in 0..NUM_DET {
        let base = i * ROW_LEN;
        if base + ROW_LEN > det.len() {
            break;
        }
        let (x1, y1, x2, y2) = (det[base], det[base + 1], det[base + 2], det[base + 3]);
        let conf = det[base + 4];
        let cls = det[base + 5] as i32;
        let min_conf = if cls == 1 { CONF_TEXT } else { CONF_OTHER };
        if conf < min_conf {
            continue;
        }
        if x2 - x1 < MIN_BOX_DIM || y2 - y1 < MIN_BOX_DIM {
            continue;
        }
        let mut coeff = [0.0f32; 32];
        coeff.copy_from_slice(&det[base + 6..base + 38]);
        out.push(RawDet {
            x1,
            y1,
            x2,
            y2,
            conf,
            cls,
            coeff,
        });
    }
    out
}

#[cfg(feature = "ort")]
fn run_session(
    session: &mut ort::session::Session,
    chw: Vec<f32>,
) -> Result<(Vec<f32>, Vec<f32>), AppError> {
    use ndarray::Array;
    let input_name = session
        .inputs
        .first()
        .map(|i| i.name.clone())
        .ok_or_else(|| err("model tanpa input".into()))?;
    let arr = Array::from_shape_vec((1, 3, MODEL_INPUT as usize, MODEL_INPUT as usize), chw)
        .map_err(|e| err(format!("bentuk tensor: {e}")))?;
    let tensor = ort::value::Tensor::from_array(arr).map_err(|e| err(format!("tensor: {e}")))?;
    let outputs = session
        .run(ort::inputs![input_name => tensor])
        .map_err(|e| err(format!("inferensi: {e}")))?;
    let (_, d0) = outputs[0]
        .try_extract_tensor::<f32>()
        .map_err(|e| err(format!("output0: {e}")))?;
    let (_, d1) = outputs[1]
        .try_extract_tensor::<f32>()
        .map_err(|e| err(format!("output1: {e}")))?;
    Ok((d0.to_vec(), d1.to_vec()))
}

/// Detector: path model + sesi `ort` CPU malas (dibuka saat deteksi pertama).
pub struct BubbleDetector {
    model_path: PathBuf,
    #[cfg(feature = "ort")]
    session: Mutex<Option<ort::session::Session>>,
}

impl BubbleDetector {
    pub fn new(model_path: PathBuf) -> Self {
        Self {
            model_path,
            #[cfg(feature = "ort")]
            session: Mutex::new(None),
        }
    }

    pub fn model_path(&self) -> &Path {
        &self.model_path
    }

    /// Pastikan model ada; unduh dari repo mobile bila belum ada (42MB, sekali).
    pub async fn ensure_model(
        &self,
        http: &crate::network::HttpClientManager,
    ) -> Result<(), AppError> {
        if self.model_path.exists() {
            return Ok(());
        }
        if let Some(parent) = self.model_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let bytes = http
            .client()
            .get(BUBBLE_MODEL_URL)
            .send()
            .await?
            .error_for_status()
            .map_err(reqwest::Error::from)?
            .bytes()
            .await?;
        if bytes.is_empty() {
            return Err(AppError::Network("model bubble kosong".into()));
        }
        std::fs::write(&self.model_path, &bytes)?;
        Ok(())
    }

    /// Deteksi bubble dari bytes gambar. Tanpa fitur `ort` = error jelas.
    pub fn detect(&self, data: &[u8]) -> Result<Vec<BubbleBox>, AppError> {
        #[cfg(not(feature = "ort"))]
        {
            let _ = data;
            return Err(AppError::Validation(
                "deteksi bubble butuh build fitur `ort` + model 42MB (lihat 5.2)".into(),
            ));
        }
        #[cfg(feature = "ort")]
        {
            use ort::session::builder::GraphOptimizationLevel;
            let img = image::load_from_memory(data).map_err(|e| err(format!("decode: {e}")))?;
            let (chw, scale, pad_l, pad_t, ow, oh) = letterbox(&img);
            let mut guard = self
                .session
                .lock()
                .map_err(|e| err(format!("kunci sesi: {e}")))?;
            if guard.is_none() {
                if !self.model_path.exists() {
                    return Err(AppError::Validation(format!(
                        "model bubble belum ada di {} — panggil ensure_model dulu",
                        self.model_path.display()
                    )));
                }
                let sess = ort::session::Session::builder()
                    .map_err(|e| err(format!("ort: {e}")))?
                    .with_optimization_level(GraphOptimizationLevel::Level3)
                    .map_err(|e| err(format!("ort: {e}")))?
                    .with_intra_threads(4)
                    .map_err(|e| err(format!("ort: {e}")))?
                    .commit_from_file(&self.model_path)
                    .map_err(|e| err(format!("muat model: {e}")))?;
                *guard = Some(sess);
            }
            let session = guard.as_mut().expect("sesi terisi");
            let (det, proto) = run_session(session, chw)?;
            let kept = nms(extract_candidates(&det));
            Ok(kept
                .iter()
                .map(|d| to_bubble(d, &proto, ow, oh, scale, pad_l, pad_t))
                .collect())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(x1: f32, y1: f32, x2: f32, y2: f32, conf: f32, cls: i32) -> RawDet {
        RawDet {
            x1,
            y1,
            x2,
            y2,
            conf,
            cls,
            coeff: [0.0; 32],
        }
    }

    #[test]
    fn nms_menekan_overlap_kelas_sama_045() {
        let dets = vec![
            raw(0.0, 0.0, 100.0, 100.0, 0.9, 2),
            raw(10.0, 10.0, 90.0, 90.0, 0.8, 2),
            raw(200.0, 200.0, 300.0, 300.0, 0.7, 2),
        ];
        let kept = nms(dets);
        assert_eq!(kept.len(), 2);
        assert_eq!(kept[0].conf, 0.9);
        assert_eq!(kept[1].conf, 0.7);
    }

    #[test]
    fn nms_tidak_menekan_kelas_beda() {
        let dets = vec![
            raw(0.0, 0.0, 100.0, 100.0, 0.9, 2),
            raw(10.0, 10.0, 90.0, 90.0, 0.8, 1),
        ];
        assert_eq!(nms(dets).len(), 2);
    }

    #[test]
    fn iou_kotak_identik_satu() {
        let d = raw(0.0, 0.0, 50.0, 40.0, 0.9, 2);
        assert!((iou(&d, &d) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn extract_menghormati_ambang_conf_per_kelas() {
        // 1 det: cls 1 conf 0.2 (lolos 0.15), cls 2 conf 0.2 (gugur 0.25).
        let mut det = vec![0.0f32; NUM_DET * ROW_LEN];
        det[0..6].copy_from_slice(&[10.0, 10.0, 60.0, 60.0, 0.2, 1.0]);
        let base = ROW_LEN;
        det[base..base + 6].copy_from_slice(&[10.0, 10.0, 60.0, 60.0, 0.2, 2.0]);
        let cands = extract_candidates(&det);
        assert_eq!(cands.len(), 1);
        assert_eq!(cands[0].cls, 1);
    }

    #[test]
    fn letterbox_strip_nyata_tanpa_stretch() {
        let bytes: &[u8] = include_bytes!("../../../test/fixtures/image-webtoon.jpeg");
        let img = image::load_from_memory(bytes).unwrap();
        assert_eq!(img.dimensions(), (800, 4710));
        let (chw, scale, pad_l, pad_t, ow, oh) = letterbox(&img);
        assert_eq!(chw.len(), 3 * 1280 * 1280);
        assert!((scale - 1280.0 / 4710.0).abs() < 1e-4);
        assert_eq!((ow, oh), (800, 4710));
        // 800*scale ≈ 217 → padding kiri ≈ (1280−217)/2.
        assert!((pad_l - (1280.0 - 800.0 * scale) / 2.0).abs() < 1.5);
        assert!(pad_t.abs() < 1.5);
    }

    #[test]
    fn tanpa_model_error_jelas_bukan_kosong() {
        let dir = std::env::temp_dir().join(format!("kuron-bubble-{}", std::process::id()));
        let det = BubbleDetector::new(dir.join("bubble_detector.onnx"));
        // Tanpa fitur `ort`: error validasi jelas. Dengan `ort` (default):
        // decode gagal duluan — tetap error jelas, bukan hasil kosong.
        match det.detect(b"not-an-image") {
            #[cfg(not(feature = "ort"))]
            Err(AppError::Validation(m)) => assert!(m.contains("ort") || m.contains("model")),
            #[cfg(feature = "ort")]
            Err(AppError::Internal(m)) => assert!(m.contains("decode")),
            Err(e) => panic!("jenis error salah: {e}"),
            Ok(_) => panic!("seharusnya error"),
        }
        std::fs::remove_dir_all(&dir).ok();
    }
}
