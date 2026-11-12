//! Screen media: still images and GIF animations fitted to each LCD.

use crate::drivers::akp05e_protocol::{draw_text_centered, orient_for_panel, parse_hex_color};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use image::{imageops::FilterType, Rgb, RgbImage, RgbaImage};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Cursor;
use std::path::PathBuf;

const MAX_BYTES: usize = 8 * 1024 * 1024;
const MAX_EDGE: u32 = 1600;
const MAX_STORED_FRAMES: usize = 24;
const MAX_DECODED_FRAMES: usize = 80;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaPreview {
    pub id: String,
    pub file_name: String,
    pub kind: String,
    pub width: u32,
    pub height: u32,
    pub frame_count: u32,
    pub source_frame_count: u32,
    pub frame_delay_ms: u32,
    pub delays_ms: Vec<u32>,
    pub frames: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct StagedClip {
    pub id: String,
    pub file_name: String,
    pub kind: String,
    pub width: u32,
    pub height: u32,
    pub source_frame_count: u32,
    pub delays_ms: Vec<u32>,
    pub frames: Vec<RgbaImage>,
    pub preview: MediaPreview,
}

#[derive(Debug, Clone)]
pub struct RenderedFrames {
    pub jpegs: Vec<Vec<u8>>,
    pub delays_ms: Vec<u64>,
    pub loop_playback: bool,
}

pub struct ZonePaint<'a> {
    pub label: &'a str,
    pub value: &'a str,
    pub accent: &'a str,
    pub media_id: Option<&'a str>,
    pub fit: &'a str,
    pub speed: f32,
    pub loop_playback: bool,
}

#[derive(Debug)]
pub struct MediaStore {
    clips: HashMap<String, StagedClip>,
    directory: PathBuf,
}

impl Default for MediaStore {
    fn default() -> Self { Self { clips: HashMap::new(), directory: media_dir() } }
}

#[derive(Serialize, Deserialize)]
struct DiskMeta {
    #[serde(default)]
    data_file: Option<String>,
    file_name: String,
    kind: String,
    delay_ms: u32,
}

impl MediaStore {
    pub fn load() -> Self {
        Self::load_from(media_dir())
    }

    fn load_from(dir: PathBuf) -> Self {
        let mut store = Self { clips: HashMap::new(), directory: dir.clone() };
        let Ok(entries) = fs::read_dir(&dir) else {
            return store;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let Some(id) = path.file_stem().and_then(|s| s.to_str()).map(|s| s.to_string()) else {
                continue;
            };
            let Ok(meta_raw) = fs::read_to_string(&path) else { continue };
            let Ok(meta) = serde_json::from_str::<DiskMeta>(&meta_raw) else { continue };
            let data_file = match meta.data_file.as_deref() {
                Some(name) if valid_blob_name(name) => name.to_string(),
                Some(_) => continue,
                None => format!("{id}.bin"), // Existing installations retain their media.
            };
            let bin_path = dir.join(data_file);
            let Ok(bytes) = fs::read(&bin_path) else { continue };
            if let Ok(clip) = decode_into_clip(&id, &meta.file_name, &bytes, &meta.kind, meta.delay_ms) {
                store.clips.insert(id, clip);
            }
        }
        store
    }

    pub fn previews(&self) -> Vec<MediaPreview> {
        let mut items: Vec<MediaPreview> = self.clips.values().map(|c| c.preview.clone()).collect();
        items.sort_by(|a, b| a.file_name.cmp(&b.file_name));
        items
    }

    pub fn stage_bytes(&mut self, id: &str, file_name: &str, bytes: &[u8]) -> Result<MediaPreview, String> {
        if bytes.len() > MAX_BYTES {
            return Err("File is larger than 8 MB. Export a smaller GIF or image.".into());
        }
        let id = sanitize_id(id)?;
        let clip = decode_into_clip(&id, file_name, bytes, "file", 100)?;
        let preview = clip.preview.clone();
        persist_clip(&self.directory, &id, file_name, "file", clip.preview.frame_delay_ms, bytes)
            .map_err(|err| format!("Media was not saved: {err}"))?;
        self.clips.insert(id, clip);
        Ok(preview)
    }

    pub fn stage_sequence(
        &mut self,
        id: &str,
        file_name: &str,
        frames_b64: &[String],
        delay_ms: u32,
    ) -> Result<MediaPreview, String> {
        if frames_b64.is_empty() {
            return Err("Animation needs at least one frame.".into());
        }
        if frames_b64.len() > MAX_STORED_FRAMES {
            return Err(format!("Animations are limited to {MAX_STORED_FRAMES} frames."));
        }
        let id = sanitize_id(id)?;
        let mut packed = Vec::new();
        packed.extend_from_slice(b"DFF1");
        packed.extend_from_slice(&(frames_b64.len() as u32).to_le_bytes());
        packed.extend_from_slice(&delay_ms.max(40).to_le_bytes());
        for raw in frames_b64 {
            let bytes = decode_b64(raw)?;
            if bytes.len() > MAX_BYTES {
                return Err("A frame is larger than 8 MB.".into());
            }
            packed.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
            packed.extend_from_slice(&bytes);
        }
        let clip = decode_into_clip(&id, file_name, &packed, "sequence", delay_ms)?;
        let preview = clip.preview.clone();
        persist_clip(&self.directory, &id, file_name, "sequence", delay_ms, &packed)
            .map_err(|err| format!("Animation was not saved: {err}"))?;
        self.clips.insert(id, clip);
        Ok(preview)
    }

    pub fn render_fitted(
        &self,
        id: &str,
        width: u32,
        height: u32,
        rotation: u16,
        fit: &str,
        bg: [u8; 3],
        title: Option<&str>,
        speed: f32,
        loop_playback: bool,
    ) -> Option<RenderedFrames> {
        let clip = self.clips.get(id)?;
        if width == 0 || height == 0 || clip.frames.is_empty() {
            return None;
        }
        let mut jpegs = Vec::with_capacity(clip.frames.len());
        let mut delays = Vec::with_capacity(clip.frames.len());
        for (frame, delay) in clip.frames.iter().zip(clip.delays_ms.iter()) {
            let mut canvas = fit_onto(frame, width, height, fit, bg);
            if let Some(text) = title {
                if !text.trim().is_empty() {
                    paint_title(&mut canvas, text);
                }
            }
            let oriented = orient_for_panel(canvas, rotation);
            jpegs.push(encode_jpeg(&oriented, 90));
            delays.push(pace_delay(*delay, speed));
        }
        Some(RenderedFrames {
            jpegs,
            delays_ms: delays,
            loop_playback: loop_playback && clip.frames.len() > 1,
        })
    }

    /// Paints the touch strip. Zone images replace that zone's card; the rest stay labeled.
    pub fn render_zone_strip(
        &self,
        width: u32,
        height: u32,
        rotation: u16,
        zones: &[ZonePaint<'_>],
    ) -> RenderedFrames {
        let steps = zones
            .iter()
            .filter_map(|z| {
                z.media_id
                    .and_then(|id| self.clips.get(id))
                    .map(|c| c.frames.len())
            })
            .max()
            .unwrap_or(1)
            .clamp(1, MAX_STORED_FRAMES);

        let delay = zones
            .iter()
            .filter_map(|z| {
                let id = z.media_id?;
                let clip = self.clips.get(id)?;
                let base = clip.delays_ms.first().copied().unwrap_or(100);
                Some(pace_delay(base, z.speed))
            })
            .min()
            .unwrap_or(100);

        let loop_playback = steps > 1
            && zones.iter().any(|z| {
                z.loop_playback
                    && z.media_id
                        .and_then(|id| self.clips.get(id))
                        .map(|c| c.frames.len() > 1)
                        .unwrap_or(false)
            });

        let mut jpegs = Vec::with_capacity(steps);
        for step in 0..steps {
            let mut img = paint_strip_frame(self, width, height, zones, step);
            img = orient_for_panel(img, rotation);
            jpegs.push(encode_jpeg(&img, 88));
        }
        RenderedFrames {
            jpegs,
            delays_ms: vec![delay; steps],
            loop_playback,
        }
    }

    /// Crops one equal slice of a full-strip image into a single touch window.
    pub fn render_strip_window(
        &self,
        id: &str,
        strip_width: u32,
        strip_height: u32,
        window_index: u32,
        window_count: u32,
        out_width: u32,
        out_height: u32,
        rotation: u16,
        fit: &str,
        speed: f32,
        loop_playback: bool,
    ) -> Option<RenderedFrames> {
        let clip = self.clips.get(id)?;
        if strip_width == 0
            || strip_height == 0
            || out_width == 0
            || out_height == 0
            || clip.frames.is_empty()
        {
            return None;
        }
        let windows = window_count.max(1);
        let index = window_index.min(windows - 1);
        let mut jpegs = Vec::with_capacity(clip.frames.len());
        let mut delays = Vec::with_capacity(clip.frames.len());
        for (frame, delay) in clip.frames.iter().zip(clip.delays_ms.iter()) {
            let strip = fit_onto(frame, strip_width, strip_height, fit, [13, 16, 24]);
            let slice_w = (strip_width / windows).max(1);
            let x = (index * slice_w).min(strip_width.saturating_sub(1));
            let w = slice_w.min(strip_width.saturating_sub(x)).max(1);
            let h = strip_height.min(strip.height()).max(1);
            let cropped = image::imageops::crop_imm(&strip, x, 0, w, h).to_image();
            let rgba = RgbaImage::from_fn(cropped.width(), cropped.height(), |px, py| {
                let p = cropped.get_pixel(px, py);
                image::Rgba([p[0], p[1], p[2], 255])
            });
            let window = fit_onto(&rgba, out_width, out_height, "cover", [13, 16, 24]);
            let oriented = orient_for_panel(window, rotation);
            jpegs.push(encode_jpeg(&oriented, 88));
            delays.push(pace_delay(*delay, speed));
        }
        Some(RenderedFrames {
            jpegs,
            delays_ms: delays,
            loop_playback: loop_playback && clip.frames.len() > 1,
        })
    }
}

fn paint_strip_frame(
    store: &MediaStore,
    width: u32,
    height: u32,
    zones: &[ZonePaint<'_>],
    step: usize,
) -> RgbImage {
    let mut img: RgbImage = image::ImageBuffer::from_pixel(width, height, Rgb([13, 16, 24]));
    let zone_count = zones.len().max(1) as u32;
    let zone_w = width / zone_count;
    for (idx, zone) in zones.iter().enumerate() {
        let zx0 = (idx as u32) * zone_w + 6;
        let zx1 = ((idx as u32 + 1) * zone_w).saturating_sub(6).max(zx0 + 2);
        let zy0 = 8u32.min(height.saturating_sub(2));
        let zy1 = height.saturating_sub(8).max(zy0 + 2);
        let accent = parse_hex_color(zone.accent, [139, 92, 246]);
        let clip = zone.media_id.and_then(|id| store.clips.get(id));
        let has_image = clip.map(|c| !c.frames.is_empty()).unwrap_or(false);
        if let Some(clip) = clip {
            if !clip.frames.is_empty() {
                let frame = &clip.frames[step % clip.frames.len()];
                let inner_w = zx1.saturating_sub(zx0).saturating_sub(2).max(1);
                let inner_h = zy1.saturating_sub(zy0).saturating_sub(2).max(1);
                let fitted = fit_onto(frame, inner_w, inner_h, zone.fit, [20, 25, 36]);
                blit_rgb(&mut img, zx0 + 1, zy0 + 1, &fitted);
            }
        } else {
            for y in zy0..zy1 {
                for x in zx0..zx1 {
                    let border = x == zx0 || x + 1 == zx1 || y == zy0 || y + 1 == zy1;
                    if border {
                        img.put_pixel(x, y, Rgb([38, 45, 64]));
                    } else {
                        img.put_pixel(x, y, Rgb([20, 25, 36]));
                    }
                }
            }
            let center_x = ((zx0 + zx1) / 2) as i32;
            draw_text_centered(&mut img, zone.label, center_x, (zy0 + 8) as i32, 2, Rgb([148, 156, 179]));
            draw_text_centered(
                &mut img,
                zone.value,
                center_x,
                (zy0 + 32) as i32,
                2,
                Rgb([
                    accent[0].saturating_add(30),
                    accent[1].saturating_add(30),
                    accent[2].saturating_add(30),
                ]),
            );
        }
        if has_image && !zone.label.is_empty() {
            draw_text_centered(
                &mut img,
                zone.label,
                ((zx0 + zx1) / 2) as i32,
                zy0 as i32 + 4,
                1,
                Rgb([244, 245, 250]),
            );
        }
    }
    img
}

fn blit_rgb(dest: &mut RgbImage, ox: u32, oy: u32, src: &RgbImage) {
    let (dw, dh) = dest.dimensions();
    let (sw, sh) = src.dimensions();
    for y in 0..sh {
        for x in 0..sw {
            let dx = ox + x;
            let dy = oy + y;
            if dx >= dw || dy >= dh {
                continue;
            }
            dest.put_pixel(dx, dy, *src.get_pixel(x, y));
        }
    }
}

fn decode_into_clip(
    id: &str,
    file_name: &str,
    bytes: &[u8],
    kind_hint: &str,
    sequence_delay: u32,
) -> Result<StagedClip, String> {
    let (frames, delays, source_count, native_w, native_h) = if kind_hint == "sequence" || bytes.starts_with(b"DFF1")
    {
        decode_sequence(bytes, sequence_delay)?
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        decode_gif(bytes)?
    } else {
        decode_still(bytes)?
    };
    if frames.is_empty() {
        return Err("No frames could be read from that file.".into());
    }
    let (frames, delays) = sample_frames(frames, delays);
    let (width, height) = frames[0].dimensions();
    let kind = if frames.len() > 1 { "gif" } else { "image" };
    let frame_delay = delays.first().copied().unwrap_or(100);
    let preview_frames = frames.iter().map(|f| preview_data_url(f)).collect::<Vec<_>>();
    let preview = MediaPreview {
        id: id.to_string(),
        file_name: file_name.to_string(),
        kind: kind.to_string(),
        width,
        height,
        frame_count: frames.len() as u32,
        source_frame_count: source_count,
        frame_delay_ms: frame_delay,
        delays_ms: delays.clone(),
        frames: preview_frames,
    };
    let _ = (native_w, native_h);
    Ok(StagedClip {
        id: id.to_string(),
        file_name: file_name.to_string(),
        kind: kind.to_string(),
        width,
        height,
        source_frame_count: source_count,
        delays_ms: delays,
        frames,
        preview,
    })
}

fn decode_still(bytes: &[u8]) -> Result<(Vec<RgbaImage>, Vec<u32>, u32, u32, u32), String> {
    let img = image::load_from_memory(bytes)
        .map_err(|e| format!("Could not read image: {e}"))?
        .to_rgba8();
    let (w, h) = img.dimensions();
    ensure_bounds(w, h)?;
    let stored = limit_edge(img, storage_edge(w, h));
    Ok((vec![stored], vec![100], 1, w, h))
}

fn decode_gif(bytes: &[u8]) -> Result<(Vec<RgbaImage>, Vec<u32>, u32, u32, u32), String> {
    let mut opts = gif::DecodeOptions::new();
    opts.set_color_output(gif::ColorOutput::RGBA);
    let mut decoder = opts
        .read_info(Cursor::new(bytes))
        .map_err(|e| format!("Could not read GIF: {e}"))?;
    let w = decoder.width() as u32;
    let h = decoder.height() as u32;
    ensure_bounds(w, h)?;
    let mut canvas = vec![0u8; (w as usize) * (h as usize) * 4];
    let mut produced: Vec<(RgbaImage, u32)> = Vec::new();
    let edge = storage_edge(w, h);
    loop {
        let copied = {
            let next = decoder
                .read_next_frame()
                .map_err(|e| format!("GIF frame failed: {e}"))?;
            let Some(frame) = next else { break };
            (
                frame.left as i32,
                frame.top as i32,
                frame.width as i32,
                frame.height as i32,
                frame.delay,
                frame.dispose,
                frame.buffer.clone(),
            )
        };
        let (left, top, fw, fh, delay_cs, dispose, buffer) = copied;
        let saved = if dispose == gif::DisposalMethod::Previous {
            Some(canvas.clone())
        } else {
            None
        };
        blit_rgba(&mut canvas, w as i32, h as i32, left, top, fw, fh, &buffer);
        let full = canvas_image(&canvas, w, h);
        let delay_ms = if delay_cs == 0 { 100 } else { u32::from(delay_cs).saturating_mul(10) };
        produced.push((limit_edge(full, edge), delay_ms.max(20)));
        match dispose {
            gif::DisposalMethod::Background => {
                clear_rect(&mut canvas, w as i32, h as i32, left, top, fw, fh);
            }
            gif::DisposalMethod::Previous => {
                if let Some(prev) = saved {
                    canvas = prev;
                }
            }
            _ => {}
        }
        if produced.len() >= MAX_DECODED_FRAMES {
            break;
        }
    }
    if produced.is_empty() {
        return Err("GIF has no frames.".into());
    }
    let source = produced.len() as u32;
    let (frames, delays) = produced.into_iter().unzip();
    Ok((frames, delays, source, w, h))
}

fn decode_sequence(
    bytes: &[u8],
    delay_ms: u32,
) -> Result<(Vec<RgbaImage>, Vec<u32>, u32, u32, u32), String> {
    if bytes.len() < 12 || &bytes[0..4] != b"DFF1" {
        return Err("Animation frame pack is invalid.".into());
    }
    let count = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
    let delay = u32::from_le_bytes(bytes[8..12].try_into().unwrap()).max(delay_ms).max(40);
    let mut offset = 12usize;
    let mut frames = Vec::new();
    for _ in 0..count {
        if offset + 4 > bytes.len() {
            break;
        }
        let len = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        offset += 4;
        if offset + len > bytes.len() {
            return Err("Animation frame pack is truncated.".into());
        }
        let slice = &bytes[offset..offset + len];
        offset += len;
        let img = image::load_from_memory(slice)
            .map_err(|e| format!("Could not read animation frame: {e}"))?
            .to_rgba8();
        let (w, h) = img.dimensions();
        ensure_bounds(w, h)?;
        frames.push(limit_edge(img, storage_edge(w, h)));
    }
    if frames.is_empty() {
        return Err("Animation has no frames.".into());
    }
    let (w, h) = frames[0].dimensions();
    let n = frames.len() as u32;
    Ok((frames, vec![delay; n as usize], n, w, h))
}

fn sample_frames(frames: Vec<RgbaImage>, delays: Vec<u32>) -> (Vec<RgbaImage>, Vec<u32>) {
    if frames.len() <= MAX_STORED_FRAMES {
        return (frames, delays);
    }
    let n = frames.len();
    let mut out_frames = Vec::with_capacity(MAX_STORED_FRAMES);
    let mut out_delays = Vec::with_capacity(MAX_STORED_FRAMES);
    for i in 0..MAX_STORED_FRAMES {
        let start = i * n / MAX_STORED_FRAMES;
        let end = (((i + 1) * n / MAX_STORED_FRAMES).max(start + 1)).min(n);
        let delay = delays[start..end]
            .iter()
            .fold(0u32, |acc, d| acc.saturating_add(*d))
            .max(20);
        out_frames.push(frames[start].clone());
        out_delays.push(delay);
    }
    (out_frames, out_delays)
}

fn fit_onto(src: &RgbaImage, tw: u32, th: u32, fit: &str, bg: [u8; 3]) -> RgbImage {
    let (sw, sh) = src.dimensions();
    if sw == 0 || sh == 0 || tw == 0 || th == 0 {
        return RgbImage::from_pixel(tw.max(1), th.max(1), Rgb(bg));
    }
    let (dw, dh, ox, oy) = match fit {
        "contain" => {
            let s = (tw as f32 / sw as f32).min(th as f32 / sh as f32);
            let dw = ((sw as f32) * s).round().max(1.0) as u32;
            let dh = ((sh as f32) * s).round().max(1.0) as u32;
            (dw, dh, (tw as i32 - dw as i32) / 2, (th as i32 - dh as i32) / 2)
        }
        "stretch" | "fill" => (tw, th, 0, 0),
        _ => {
            let s = (tw as f32 / sw as f32).max(th as f32 / sh as f32);
            let dw = ((sw as f32) * s).round().max(1.0) as u32;
            let dh = ((sh as f32) * s).round().max(1.0) as u32;
            (dw, dh, (tw as i32 - dw as i32) / 2, (th as i32 - dh as i32) / 2)
        }
    };
    let scaled = image::imageops::resize(src, dw, dh, FilterType::Triangle);
    let mut canvas = RgbImage::from_pixel(tw, th, Rgb(bg));
    for y in 0..dh {
        for x in 0..dw {
            let dx = ox + x as i32;
            let dy = oy + y as i32;
            if dx < 0 || dy < 0 || dx >= tw as i32 || dy >= th as i32 {
                continue;
            }
            let p = scaled.get_pixel(x, y).0;
            let a = p[3] as f32 / 255.0;
            let dst = canvas.get_pixel(dx as u32, dy as u32).0;
            let mix = |c: u8, b: u8| -> u8 { (c as f32 * a + b as f32 * (1.0 - a)).round() as u8 };
            canvas.put_pixel(dx as u32, dy as u32, Rgb([mix(p[0], dst[0]), mix(p[1], dst[1]), mix(p[2], dst[2])]));
        }
    }
    canvas
}

fn paint_title(img: &mut RgbImage, title: &str) {
    let w = img.width();
    let h = img.height();
    if h < 16 {
        return;
    }
    let bar = (h / 5).clamp(12, 28);
    for y in (h - bar)..h {
        for x in 0..w {
            let p = img.get_pixel(x, y).0;
            img.put_pixel(x, y, Rgb([p[0] / 3, p[1] / 3, p[2] / 3]));
        }
    }
    let scale: i32 = if w >= 140 { 2 } else { 1 };
    draw_text_centered(
        img,
        title,
        (w / 2) as i32,
        h as i32 - bar as i32 + 3,
        scale,
        Rgb([244, 245, 250]),
    );
}

fn preview_data_url(frame: &RgbaImage) -> String {
    let fitted = fit_onto(frame, 160, 160, "contain", [21, 25, 37]);
    let jpeg = encode_jpeg(&fitted, 72);
    format!("data:image/jpeg;base64,{}", STANDARD.encode(jpeg))
}

fn encode_jpeg(img: &RgbImage, quality: u8) -> Vec<u8> {
    let mut out = Cursor::new(Vec::with_capacity(4096));
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, quality);
    let _ = encoder.encode_image(img);
    out.into_inner()
}

pub fn pace_delay(ms: u32, speed: f32) -> u64 {
    let speed = if speed < 0.25 {
        0.25
    } else if speed > 4.0 {
        4.0
    } else {
        speed
    };
    ((ms.max(20) as f32) / speed).clamp(80.0, 2000.0) as u64
}

fn storage_edge(w: u32, h: u32) -> u32 {
    let aspect = w as f32 / h.max(1) as f32;
    if !(0.45..=2.2).contains(&aspect) {
        800
    } else {
        360
    }
}

fn limit_edge(img: RgbaImage, max_edge: u32) -> RgbaImage {
    let (w, h) = img.dimensions();
    if w.max(h) <= max_edge {
        return img;
    }
    let s = max_edge as f32 / w.max(h) as f32;
    let nw = ((w as f32) * s).round().max(1.0) as u32;
    let nh = ((h as f32) * s).round().max(1.0) as u32;
    image::imageops::resize(&img, nw, nh, FilterType::Triangle)
}

fn ensure_bounds(w: u32, h: u32) -> Result<(), String> {
    if w == 0 || h == 0 {
        return Err("Image has no pixels.".into());
    }
    if w > MAX_EDGE || h > MAX_EDGE {
        return Err(format!(
            "This image is {w}×{h}. Export it at {MAX_EDGE}px or smaller so it fits the key screens."
        ));
    }
    Ok(())
}

fn blit_rgba(
    canvas: &mut [u8],
    cw: i32,
    ch: i32,
    left: i32,
    top: i32,
    fw: i32,
    fh: i32,
    buffer: &[u8],
) {
    if fw <= 0 || fh <= 0 {
        return;
    }
    for y in 0..fh {
        for x in 0..fw {
            let dx = left + x;
            let dy = top + y;
            if dx < 0 || dy < 0 || dx >= cw || dy >= ch {
                continue;
            }
            let si = ((y as usize) * (fw as usize) + (x as usize)) * 4;
            if si + 3 >= buffer.len() {
                continue;
            }
            let di = ((dy as usize) * (cw as usize) + (dx as usize)) * 4;
            let sa = buffer[si + 3] as u32;
            if sa == 0 {
                continue;
            }
            if sa >= 255 {
                canvas[di..di + 4].copy_from_slice(&buffer[si..si + 4]);
            } else {
                let inv = 255 - sa;
                for c in 0..3 {
                    canvas[di + c] =
                        ((buffer[si + c] as u32 * sa + canvas[di + c] as u32 * inv) / 255) as u8;
                }
                canvas[di + 3] =
                    (sa + (canvas[di + 3] as u32 * inv / 255)).min(255) as u8;
            }
        }
    }
}

fn clear_rect(canvas: &mut [u8], cw: i32, ch: i32, left: i32, top: i32, fw: i32, fh: i32) {
    for y in 0..fh.max(0) {
        for x in 0..fw.max(0) {
            let dx = left + x;
            let dy = top + y;
            if dx < 0 || dy < 0 || dx >= cw || dy >= ch {
                continue;
            }
            let di = ((dy as usize) * (cw as usize) + (dx as usize)) * 4;
            canvas[di..di + 4].fill(0);
        }
    }
}

fn canvas_image(canvas: &[u8], w: u32, h: u32) -> RgbaImage {
    RgbaImage::from_raw(w, h, canvas.to_vec()).unwrap_or_else(|| RgbaImage::new(w, h))
}

pub fn decode_b64(input: &str) -> Result<Vec<u8>, String> {
    let raw = input.split(',').next_back().unwrap_or(input).trim();
    STANDARD
        .decode(raw)
        .map_err(|_| "Image data was not valid base64.".to_string())
}

fn sanitize_id(id: &str) -> Result<String, String> {
    let clean: String = id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(64)
        .collect();
    if clean.is_empty() {
        Err("Missing media id.".into())
    } else {
        Ok(clean)
    }
}

fn media_dir() -> PathBuf {
    let base = std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir());
    base.join("DeckForge").join("media")
}

fn valid_blob_name(name: &str) -> bool {
    name.strip_suffix(".bin").is_some_and(|hash| hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()))
}

fn persist_clip(dir: &std::path::Path, id: &str, file_name: &str, kind: &str, delay_ms: u32, bytes: &[u8]) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    // Publish new bytes under an immutable content hash, then atomically switch
    // the metadata pointer. A failed replacement cannot corrupt the old clip.
    let hash = crate::drivers::frame_cache::AntiFreezeFrameCache::compute_sha256_hex(bytes);
    let data_file = format!("{hash}.bin");
    crate::db::write_atomic_bytes(&dir.join(&data_file), bytes)?;
    let meta = DiskMeta {
        data_file: Some(data_file), file_name: file_name.to_string(), kind: kind.to_string(), delay_ms,
    };
    let raw = serde_json::to_string(&meta).map_err(|e| e.to_string())?;
    crate::db::write_atomic(&dir.join(format!("{id}.json")), &raw)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestMediaDir(PathBuf);

    impl TestMediaDir {
        fn new() -> Self {
            static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let id = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let dir = std::env::temp_dir().join(format!("deckforge-media-{}-{}-{id}", std::process::id(), crate::drivers::hal::now_ms()));
            fs::create_dir(&dir).unwrap();
            Self(dir)
        }
        fn store(&self) -> MediaStore { MediaStore::load_from(self.0.clone()) }
    }

    impl Drop for TestMediaDir {
        fn drop(&mut self) {
            // Only direct test-created files and empty failure-injection directories.
            if let Ok(entries) = fs::read_dir(&self.0) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() { let _ = fs::remove_dir(path); }
                    else { let _ = fs::remove_file(path); }
                }
            }
            let _ = fs::remove_dir(&self.0);
        }
    }

    #[test]
    fn failed_media_save_is_not_acknowledged_or_added_to_memory() {
        let dir = TestMediaDir::new();
        fs::write(dir.0.join("blocked"), b"not a directory").unwrap();
        let mut store = MediaStore::load_from(dir.0.join("blocked"));
        assert!(store.stage_bytes("photo", "photo.gif", &tiny_gif()).unwrap_err().contains("not saved"));
        let frame = STANDARD.encode(encode_jpeg(&RgbImage::new(4, 4), 90));
        assert!(store.stage_sequence("animation", "animation", &[frame], 80).is_err());
        assert!(store.previews().is_empty());
    }

    #[test]
    fn metadata_failure_preserves_previous_media_in_memory_and_after_reload() {
        let dir = TestMediaDir::new();
        let mut store = dir.store();
        store.stage_bytes("clip", "original.gif", &tiny_gif()).unwrap();
        fs::create_dir(dir.0.join("clip.json.tmp")).unwrap();
        let replacement = encode_jpeg(&RgbImage::new(4, 4), 90);
        assert!(store.stage_bytes("clip", "replacement.jpg", &replacement).is_err());
        assert_eq!(store.previews()[0].file_name, "original.gif");
        assert_eq!(dir.store().previews()[0].file_name, "original.gif");
        fs::remove_dir(dir.0.join("clip.json.tmp")).unwrap();
        store.stage_bytes("clip", "replacement.jpg", &replacement).unwrap();
        assert_eq!(dir.store().previews()[0].file_name, "replacement.jpg");
    }

    #[test]
    fn sequences_and_legacy_media_survive_reload() {
        let dir = TestMediaDir::new();
        fs::write(dir.0.join("legacy.bin"), tiny_gif()).unwrap();
        fs::write(dir.0.join("legacy.json"), r#"{"file_name":"old.gif","kind":"file","delay_ms":80}"#).unwrap();
        let mut store = dir.store();
        assert_eq!(store.previews()[0].file_name, "old.gif");
        let frame = STANDARD.encode(encode_jpeg(&RgbImage::new(4, 4), 90));
        store.stage_sequence("sequence", "sequence", &[frame.clone(), frame], 100).unwrap();
        let reloaded = dir.store();
        assert_eq!(reloaded.previews().len(), 2);
        assert!(reloaded.render_fitted("sequence", 112, 112, 0, "cover", [0, 0, 0], None, 1.0, true).is_some());
        assert!(!valid_blob_name("../outside.bin"));
    }

    fn tiny_gif() -> Vec<u8> {
        let mut buf = Vec::new();
        {
            let mut encoder = gif::Encoder::new(&mut buf, 4, 4, &[]).unwrap();
            encoder.set_repeat(gif::Repeat::Infinite).unwrap();
            for color in [[255u8, 40, 40, 255], [40u8, 80, 255, 255]] {
                let mut pixels = vec![0u8; 4 * 4 * 4];
                for px in pixels.chunks_mut(4) {
                    px.copy_from_slice(&color);
                }
                let mut frame = gif::Frame::from_rgba(4, 4, &mut pixels);
                frame.delay = 8;
                encoder.write_frame(&frame).unwrap();
            }
        }
        buf
    }

    #[test]
    fn gif_decodes_and_fits_every_screen_size() {
        let dir = TestMediaDir::new();
        let mut store = dir.store();
        let preview = store.stage_bytes("pulse", "pulse.gif", &tiny_gif()).unwrap();
        assert_eq!(preview.kind, "gif");
        assert_eq!(preview.frame_count, 2);
        for (w, h, rotation) in [(112u32, 112u32, 180u16), (72, 72, 0), (96, 96, 0), (85, 85, 90)] {
            let rendered = store
                .render_fitted("pulse", w, h, rotation, "cover", [10, 12, 20], Some("LIVE"), 1.0, true)
                .unwrap();
            assert_eq!(rendered.jpegs.len(), 2, "{w}x{h}");
            assert_eq!(&rendered.jpegs[0][0..2], &[0xFF, 0xD8]);
            assert!(rendered.loop_playback);
            assert!(rendered.delays_ms[0] >= 80);
        }
        let strip = store
            .render_fitted("pulse", 800, 100, 180, "cover", [13, 16, 24], None, 2.0, true)
            .unwrap();
        assert_eq!(strip.jpegs.len(), 2);
        assert!(&strip.jpegs[0][0..2] == &[0xFF, 0xD8]);
    }
}
