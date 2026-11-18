use std::path::{Path, PathBuf};
use tokio::process::Command;

pub fn get_cache_dir() -> PathBuf {
    let base = if let Some(xdg) = std::env::var_os("XDG_CACHE_HOME") {
        if !xdg.is_empty() {
            PathBuf::from(xdg)
        } else {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
            PathBuf::from(home).join(".cache")
        }
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        PathBuf::from(home).join(".cache")
    };
    base.join("aura/thumbs")
}

pub fn get_scaled_cache_dir() -> PathBuf {
    let base = if let Some(xdg) = std::env::var_os("XDG_CACHE_HOME") {
        if !xdg.is_empty() {
            PathBuf::from(xdg)
        } else {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
            PathBuf::from(home).join(".cache")
        }
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        PathBuf::from(home).join(".cache")
    };
    base.join("aura/scaled")
}

pub fn get_cinematic_cache_dir() -> PathBuf {
    let base = if let Some(xdg) = std::env::var_os("XDG_CACHE_HOME") {
        if !xdg.is_empty() {
            PathBuf::from(xdg)
        } else {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
            PathBuf::from(home).join(".cache")
        }
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        PathBuf::from(home).join(".cache")
    };
    base.join("aura/cinematic")
}

pub fn get_cinematic_anim_cache_dir() -> PathBuf {
    let base = if let Some(xdg) = std::env::var_os("XDG_CACHE_HOME") {
        if !xdg.is_empty() {
            PathBuf::from(xdg)
        } else {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
            PathBuf::from(home).join(".cache")
        }
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        PathBuf::from(home).join(".cache")
    };
    base.join("aura/cinematic_anim")
}

pub fn get_honeycomb_cache_dir() -> PathBuf {
    let base = if let Some(xdg) = std::env::var_os("XDG_CACHE_HOME") {
        if !xdg.is_empty() {
            PathBuf::from(xdg)
        } else {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
            PathBuf::from(home).join(".cache")
        }
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        PathBuf::from(home).join(".cache")
    };
    base.join("aura/honeycomb")
}

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{LazyLock, Mutex};
use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::ImageEncoder;

#[derive(Debug)]
pub struct LruMemoryCache<K: Eq + std::hash::Hash + Clone, V: Clone> {
    capacity: usize,
    map: HashMap<K, V>,
    order: VecDeque<K>,
}

impl<K: Eq + std::hash::Hash + Clone, V: Clone> LruMemoryCache<K, V> {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            map: HashMap::with_capacity(capacity),
            order: VecDeque::with_capacity(capacity),
        }
    }

    pub fn get(&mut self, key: &K) -> Option<V> {
        if let Some(val) = self.map.get(key) {
            let val = val.clone();
            if let Some(pos) = self.order.iter().position(|k| k == key) {
                self.order.remove(pos);
            }
            self.order.push_back(key.clone());
            Some(val)
        } else {
            None
        }
    }

    pub fn insert(&mut self, key: K, val: V) {
        if self.map.contains_key(&key) {
            self.map.insert(key.clone(), val);
            if let Some(pos) = self.order.iter().position(|k| k == &key) {
                self.order.remove(pos);
            }
            self.order.push_back(key);
        } else {
            while self.order.len() >= self.capacity {
                if let Some(oldest) = self.order.pop_front() {
                    self.map.remove(&oldest);
                }
            }
            self.map.insert(key.clone(), val);
            self.order.push_back(key);
        }
    }
}

pub static THUMB_HANDLE_CACHE: LazyLock<Mutex<LruMemoryCache<String, cosmic::iced::widget::image::Handle>>> =
    LazyLock::new(|| Mutex::new(LruMemoryCache::new(96)));

static CINEMATIC_IN_FLIGHT: LazyLock<Mutex<HashSet<PathBuf>>> = LazyLock::new(|| Mutex::new(HashSet::new()));
static HONEYCOMB_IN_FLIGHT: LazyLock<Mutex<HashSet<PathBuf>>> = LazyLock::new(|| Mutex::new(HashSet::new()));
static FAILED_ANIM_CACHE: LazyLock<Mutex<HashSet<PathBuf>>> = LazyLock::new(|| Mutex::new(HashSet::new()));

const MAX_FFMPEG_CONCURRENT: usize = 2;
static FFMPEG_GATE: LazyLock<(Mutex<usize>, std::sync::Condvar)> =
    LazyLock::new(|| (Mutex::new(0), std::sync::Condvar::new()));
static ASYNC_FFMPEG_SEM: LazyLock<tokio::sync::Semaphore> =
    LazyLock::new(|| tokio::sync::Semaphore::const_new(MAX_FFMPEG_CONCURRENT));

struct FfmpegPermit;
impl FfmpegPermit {
    fn acquire() -> Self {
        let (lock, cvar) = &*FFMPEG_GATE;
        let mut active = match lock.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        while *active >= MAX_FFMPEG_CONCURRENT {
            active = match cvar.wait(active) {
                Ok(g) => g,
                Err(p) => p.into_inner(),
            };
        }
        *active += 1;
        Self
    }
}
impl Drop for FfmpegPermit {
    fn drop(&mut self) {
        let (lock, cvar) = &*FFMPEG_GATE;
        let mut active = match lock.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        *active = active.saturating_sub(1);
        cvar.notify_one();
    }
}

fn is_anim_failed(path: &Path) -> bool {
    FAILED_ANIM_CACHE.lock().map(|g| g.contains(path)).unwrap_or(false)
}

fn mark_anim_failed(path: &Path) {
    if let Ok(mut g) = FAILED_ANIM_CACHE.lock() {
        g.insert(path.to_path_buf());
    }
}

pub fn cinematic_thumb_path(media_path: &Path, is_active: bool, accent_rgb: Option<[u8; 3]>) -> PathBuf {
    let cache_dir = get_cinematic_cache_dir();
    let file_stem = media_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "media".into());
    let clean_stem: String = file_stem
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .take(50)
        .collect();
    let hash = format!("{:016x}", md5_simple(media_path.to_string_lossy().as_bytes()));
    let accent = accent_rgb.unwrap_or([58, 142, 230]);
    let suffix = if is_active {
        format!("act_{:02x}{:02x}{:02x}", accent[0], accent[1], accent[2])
    } else {
        "inact".to_string()
    };
    cache_dir.join(format!("{}_{}_{}.png", clean_stem, &hash[..8], suffix))
}

pub fn generate_cinematic_thumb_sync(
    media_path: &Path,
    is_active: bool,
    accent_rgb: Option<[u8; 3]>,
) -> Option<PathBuf> {
    let cache_dir = get_cinematic_cache_dir();
    let _ = std::fs::create_dir_all(&cache_dir);
    let cached_path = cinematic_thumb_path(media_path, is_active, accent_rgb);

    if cached_path.exists() {
        return Some(cached_path);
    }

    let std_thumb = thumb_path_for_video(media_path);
    let src_img_path = if std_thumb.exists() {
        std_thumb
    } else if media_path.exists() {
        media_path.to_path_buf()
    } else {
        return None;
    };

    let base_img = image::open(&src_img_path).ok()?;
    
    let card_w = 340.0f32;
    let card_h = 440.0f32;
    let skew = 0.32f32;
    let skew_px = (card_h * skew).round();
    let total_w = (card_w + skew_px) as u32;
    let total_h = card_h as u32;
    let radius = 16.0f32;
    let border_thick = if is_active { 3.5f32 } else { 1.5f32 };

    let accent = accent_rgb.unwrap_or([58, 142, 230]);
    let border_color = if is_active {
        [accent[0], accent[1], accent[2], 255]
    } else {
        [220, 225, 235, 90]
    };

    let scaled_src = base_img.resize_to_fill(
        card_w as u32,
        card_h as u32,
        image::imageops::FilterType::Triangle,
    ).to_rgba8();

    let mut out = image::RgbaImage::new(total_w, total_h);

    for y in 0..total_h {
        let y_f = y as f32;
        let left_edge = skew_px * (1.0 - y_f / card_h);

        for x in 0..total_w {
            let x_f = x as f32;
            let u = x_f - left_edge;
            let v = y_f;

            let d_x = (radius - u).max(0.0).max(u - (card_w - radius));
            let d_y = (radius - v).max(0.0).max(v - (card_h - radius));
            let dist = if (u < radius || u > card_w - radius) && (v < radius || v > card_h - radius) {
                (d_x * d_x + d_y * d_y).sqrt() - radius
            } else {
                (-u).max(u - card_w).max(-v).max(v - card_h)
            };

            if dist > 0.0 {
                if is_active && dist <= 14.0 {
                    let glow_factor = ((1.0 - dist / 14.0) * (1.0 - dist / 14.0)).clamp(0.0, 1.0);
                    let a = (170.0 * glow_factor) as u8;
                    out.put_pixel(x, y, image::Rgba([accent[0], accent[1], accent[2], a]));
                }
                continue;
            }

            let alpha_factor = if dist > -1.0 {
                ((1.0 - dist) * 0.5).clamp(0.0, 1.0)
            } else {
                1.0
            };

            let pixel = if dist >= -border_thick {
                let a = (border_color[3] as f32 * alpha_factor) as u8;
                image::Rgba([border_color[0], border_color[1], border_color[2], a])
            } else {
                let src_x = (u.clamp(0.0, card_w - 1.0)) as u32;
                let src_y = (v.clamp(0.0, card_h - 1.0)) as u32;
                let mut p = *scaled_src.get_pixel(src_x.min(scaled_src.width() - 1), src_y.min(scaled_src.height() - 1));
                
                if !is_active {
                    p[0] = (p[0] as f32 * 0.72) as u8;
                    p[1] = (p[1] as f32 * 0.72) as u8;
                    p[2] = (p[2] as f32 * 0.72) as u8;
                }

                p[3] = (p[3] as f32 * alpha_factor) as u8;
                p
            };

            out.put_pixel(x, y, pixel);
        }
    }

    // Direct GPU/memory cache insertion
    let handle_key = format!("cn:{}:{}:{:02x}{:02x}{:02x}", media_path.to_string_lossy(), is_active, accent[0], accent[1], accent[2]);
    let direct_handle = cosmic::iced::widget::image::Handle::from_rgba(total_w, total_h, out.as_raw().clone());
    if let Ok(mut cache) = THUMB_HANDLE_CACHE.lock() {
        cache.insert(handle_key, direct_handle);
    }

    let file = std::fs::File::create(&cached_path).ok()?;
    let writer = std::io::BufWriter::new(file);
    let encoder = PngEncoder::new_with_quality(
        writer,
        CompressionType::Fast,
        FilterType::NoFilter,
    );
    if encoder.write_image(
        out.as_raw(),
        total_w,
        total_h,
        image::ExtendedColorType::Rgba8,
    ).is_ok() {
        Some(cached_path)
    } else {
        None
    }
}

pub fn cinematic_thumb_handle(
    media_path: &Path,
    is_active: bool,
    accent_rgb: Option<[u8; 3]>,
) -> Option<cosmic::iced::widget::image::Handle> {
    let accent = accent_rgb.unwrap_or([58, 142, 230]);
    let key = format!("cn:{}:{}:{:02x}{:02x}{:02x}", media_path.to_string_lossy(), is_active, accent[0], accent[1], accent[2]);

    if let Ok(mut cache) = THUMB_HANDLE_CACHE.lock() {
        if let Some(handle) = cache.get(&key) {
            return Some(handle);
        }
    }

    let cached_path = cinematic_thumb_path(media_path, is_active, accent_rgb);
    if cached_path.exists() {
        let handle = cosmic::iced::widget::image::Handle::from_path(cached_path);
        if let Ok(mut cache) = THUMB_HANDLE_CACHE.lock() {
            cache.insert(key, handle.clone());
        }
        return Some(handle);
    }

    if tokio::runtime::Handle::try_current().is_ok() {
        trigger_cinematic_thumb_async(media_path.to_path_buf(), is_active, accent_rgb);
        None
    } else {
        let path = generate_cinematic_thumb_sync(media_path, is_active, accent_rgb)?;
        let handle = cosmic::iced::widget::image::Handle::from_path(path);
        if let Ok(mut cache) = THUMB_HANDLE_CACHE.lock() {
            cache.insert(key, handle.clone());
        }
        Some(handle)
    }
}

fn spawn_bg_task<F: FnOnce() + Send + 'static>(f: F) {
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        handle.spawn_blocking(f);
    } else {
        std::thread::spawn(f);
    }
}

pub fn trigger_cinematic_thumb_async(
    media_path: PathBuf,
    is_active: bool,
    accent_rgb: Option<[u8; 3]>,
) {
    let cached_path = cinematic_thumb_path(&media_path, is_active, accent_rgb);
    if cached_path.exists() {
        return;
    }

    let mut in_flight = match CINEMATIC_IN_FLIGHT.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    if in_flight.insert(cached_path.clone()) {
        drop(in_flight);
        spawn_bg_task(move || {
            let _ = generate_cinematic_thumb_sync(&media_path, is_active, accent_rgb);
            if let Ok(mut in_flight) = CINEMATIC_IN_FLIGHT.lock() {
                in_flight.remove(&cached_path);
            }
        });
    }
}

pub fn prewarm_cinematic_thumbs(media_paths: &[PathBuf], accent_rgb: [u8; 3]) {
    for path in media_paths {
        trigger_cinematic_thumb_async(path.clone(), true, Some(accent_rgb));
        trigger_cinematic_thumb_async(path.clone(), false, Some(accent_rgb));
    }
}

pub fn cinematic_thumb_for_media(
    media_path: &Path,
    is_active: bool,
    accent_rgb: Option<[u8; 3]>,
) -> Option<PathBuf> {
    let cached_path = cinematic_thumb_path(media_path, is_active, accent_rgb);
    if cached_path.exists() {
        return Some(cached_path);
    }

    // In asynchronous tokio environment (UI app runtime), never block the frame.
    // Trigger background generation and return None so fallback thumbnail or placeholder renders at 60 FPS.
    if tokio::runtime::Handle::try_current().is_ok() {
        trigger_cinematic_thumb_async(media_path.to_path_buf(), is_active, accent_rgb);
        None
    } else {
        // Synchronous fallback (e.g. CLI or unit tests without tokio runtime)
        generate_cinematic_thumb_sync(media_path, is_active, accent_rgb)
    }
}

pub const CINEMATIC_ANIM_FRAME_COUNT: usize = 30;
static CINEMATIC_ANIM_IN_FLIGHT: LazyLock<Mutex<HashSet<PathBuf>>> = LazyLock::new(|| Mutex::new(HashSet::new()));

pub fn cinematic_anim_frame_path(media_path: &Path, frame_idx: usize) -> PathBuf {
    let cache_dir = get_cinematic_anim_cache_dir();
    let hash = format!("{:016x}", md5_simple(media_path.to_string_lossy().as_bytes()));
    cache_dir.join(format!("cframe_{}_{}.jpg", &hash[..10], frame_idx % CINEMATIC_ANIM_FRAME_COUNT))
}

pub fn cinematic_video_anim_handle(
    media_path: &Path,
    frame_idx: usize,
) -> Option<cosmic::iced::widget::image::Handle> {
    let idx = frame_idx % CINEMATIC_ANIM_FRAME_COUNT;
    let key = format!("cin_anim:{}:{}", media_path.to_string_lossy(), idx);

    if let Ok(mut cache) = ANIM_HANDLE_CACHE.lock() {
        if let Some(handle) = cache.get(&key) {
            return Some(handle);
        }
    }

    let frame_file = cinematic_anim_frame_path(media_path, idx);
    if frame_file.exists() {
        let handle = cosmic::iced::widget::image::Handle::from_path(&frame_file);
        if let Ok(mut cache) = ANIM_HANDLE_CACHE.lock() {
            cache.insert(key, handle.clone());
        }
        return Some(handle);
    }

    if is_anim_failed(media_path) {
        return None;
    }

    let path_buf = media_path.to_path_buf();
    let mut in_flight = match CINEMATIC_ANIM_IN_FLIGHT.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    if in_flight.insert(path_buf.clone()) {
        drop(in_flight);
        spawn_bg_task(move || {
            generate_cinematic_anim_sync(&path_buf);
            if let Ok(mut guard) = CINEMATIC_ANIM_IN_FLIGHT.lock() {
                guard.remove(&path_buf);
            }
        });
    }

    None
}

pub fn generate_cinematic_anim_sync(media_path: &Path) {
    if !media_path.is_file() || is_anim_failed(media_path) {
        return;
    }
    let ext = media_path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    if ext == "png" || ext == "jpg" || ext == "jpeg" || ext == "webp" {
        return;
    }

    let cache_dir = get_cinematic_anim_cache_dir();
    let _ = std::fs::create_dir_all(&cache_dir);

    let mut all_exist = true;
    for i in 0..CINEMATIC_ANIM_FRAME_COUNT {
        let p = cinematic_anim_frame_path(media_path, i);
        if !p.exists() {
            all_exist = false;
            break;
        }
    }
    if all_exist {
        return;
    }

    let _permit = FfmpegPermit::acquire();

    let hash = format!("{:016x}", md5_simple(media_path.to_string_lossy().as_bytes()));
    let temp_dir = std::env::temp_dir().join(format!("aura_cin_{}_{}", std::process::id(), &hash[..8]));
    let _ = std::fs::create_dir_all(&temp_dir);

    let pattern = temp_dir.join("f_%02d.jpg");
    let mut cmd = std::process::Command::new("ffmpeg");
    cmd.arg("-nostdin")
        .arg("-y")
        .arg("-threads")
        .arg("1")
        .arg("-ss")
        .arg("00:00:01")
        .arg("-i")
        .arg(media_path)
        .arg("-t")
        .arg("3.0")
        .arg("-r")
        .arg("10")
        .arg("-vf")
        .arg("scale=960:-2")
        .arg("-q:v")
        .arg("3")
        .arg(&pattern)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    let _ = cmd.status();

    if !temp_dir.join("f_01.jpg").exists() {
        let mut fallback_cmd = std::process::Command::new("ffmpeg");
        fallback_cmd.arg("-nostdin")
            .arg("-y")
            .arg("-threads")
            .arg("1")
            .arg("-i")
            .arg(media_path)
            .arg("-t")
            .arg("3.0")
            .arg("-r")
            .arg("10")
            .arg("-vf")
            .arg("scale=960:-2")
            .arg("-q:v")
            .arg("3")
            .arg(&pattern)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        let _ = fallback_cmd.status();
    }

    let mut last_good_frame: Option<PathBuf> = None;
    for i in 0..CINEMATIC_ANIM_FRAME_COUNT {
        let f_path = temp_dir.join(format!("f_{:02}.jpg", i + 1));
        let dest = cinematic_anim_frame_path(media_path, i);
        if f_path.exists() {
            let _ = std::fs::copy(&f_path, &dest);
            last_good_frame = Some(dest);
        } else if let Some(ref fallback) = last_good_frame {
            let _ = std::fs::copy(fallback, &dest);
        }
    }
    if last_good_frame.is_none() {
        mark_anim_failed(media_path);
    }

    let _ = std::fs::remove_dir_all(&temp_dir);
}

pub fn prewarm_cinematic_video_anims(media_paths: &[PathBuf]) {
    for path in media_paths {
        if is_anim_failed(path) || cinematic_anim_frame_path(path, 0).exists() {
            continue;
        }
        let path_buf = path.clone();
        let mut in_flight = match CINEMATIC_ANIM_IN_FLIGHT.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        if in_flight.insert(path_buf.clone()) {
            drop(in_flight);
            spawn_bg_task(move || {
                generate_cinematic_anim_sync(&path_buf);
                if let Ok(mut guard) = CINEMATIC_ANIM_IN_FLIGHT.lock() {
                    guard.remove(&path_buf);
                }
            });
        }
    }
}

pub fn honeycomb_thumb_path(media_path: &Path, is_active: bool, accent_rgb: Option<[u8; 3]>) -> PathBuf {
    let cache_dir = get_honeycomb_cache_dir();
    let file_stem = media_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "media".into());
    let clean_stem: String = file_stem
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .take(50)
        .collect();
    let hash = format!("{:016x}", md5_simple(media_path.to_string_lossy().as_bytes()));
    let accent = accent_rgb.unwrap_or([58, 142, 230]);
    let suffix = if is_active {
        format!("act_{:02x}{:02x}{:02x}", accent[0], accent[1], accent[2])
    } else {
        "inact".to_string()
    };
    cache_dir.join(format!("glass4_{}_{}_{}.png", clean_stem, &hash[..8], suffix))
}

pub fn apply_glass_honeycomb_mask(
    src_img: &image::DynamicImage,
    is_active: bool,
    accent_rgb: Option<[u8; 3]>,
) -> image::RgbaImage {
    let card_w = 260.0f32;
    let card_h = (card_w * 1.1547005f32).round();
    let total_w = card_w as u32;
    let total_h = card_h as u32;

    let border_thickness = 12.5f32;
    let r_outer = 128.5f32;
    let inradius = r_outer - border_thickness;
    let corner_r = 7.0f32;
    let r_inner = inradius - corner_r;

    let scaled_src = src_img.resize_to_fill(
        total_w,
        total_h,
        image::imageops::FilterType::Triangle,
    ).to_rgba8();

    // Extract dominant vibrant neon color from the image to give each card
    // its own glowing crystal refraction rim matching its artwork
    let mut neon_color = accent_rgb.unwrap_or([0x3A, 0x9E, 0xF6]);
    let mut max_sat: f32 = 0.0;
    for sy in (20..total_h.saturating_sub(20)).step_by(12) {
        for sx in (20..total_w.saturating_sub(20)).step_by(12) {
            let p = scaled_src.get_pixel(sx, sy);
            let rf = p[0] as f32;
            let gf = p[1] as f32;
            let bf = p[2] as f32;
            let max_c = rf.max(gf).max(bf);
            let min_c = rf.min(gf).min(bf);
            let delta = max_c - min_c;
            let sat = if max_c > 15.0 { delta / max_c } else { 0.0 };
            let score = sat * (max_c / 255.0).powf(0.5);
            if score > max_sat && max_c > 45.0 {
                max_sat = score;
                let boost = (245.0 / max_c.max(1.0)).clamp(1.0, 2.4);
                neon_color = [
                    (rf * boost).min(255.0) as u8,
                    (gf * boost).min(255.0) as u8,
                    (bf * boost).min(255.0) as u8,
                ];
            }
        }
    }
    if max_sat < 0.12 {
        neon_color = [70, 165, 255]; // Luminous electric cyan/ice-blue fallback
    }

    let mut out = image::RgbaImage::new(total_w, total_h);

    let cx = card_w * 0.5;
    let cy = card_h * 0.5;
    let k_x = -0.866025404f32;
    let k_y = 0.5f32;
    let k_z = 0.577350269f32;

    // Pointy-topped hexagon signed distance function
    let sd_pointy_hex = |p_x: f32, p_y: f32, inr: f32| -> f32 {
        let mut x = p_y.abs();
        let mut y = p_x.abs();
        let dot = 2.0 * (k_x * x + k_y * y).min(0.0);
        x -= dot * k_x;
        y -= dot * k_y;
        let d_x = x.clamp(-k_z * inr, k_z * inr);
        let d_y = inr;
        let vx = x - d_x;
        let vy = y - d_y;
        (vx * vx + vy * vy).sqrt().copysign(if y > inr { 1.0 } else { -1.0 })
    };

    for y in 0..total_h {
        let py = y as f32 - cy;
        for x in 0..total_w {
            let px = x as f32 - cx;
            let dist = sd_pointy_hex(px, py, r_inner) - corner_r;

            if dist > border_thickness + 4.0 {
                continue;
            }

            // Outer luminous neon bloom / aura
            if dist > border_thickness {
                let bloom_dist = dist - border_thickness;
                let falloff = ((4.0 - bloom_dist) / 4.0).clamp(0.0, 1.0);
                let alpha = if is_active {
                    (220.0 * falloff * falloff) as u8
                } else {
                    (145.0 * falloff * falloff) as u8
                };
                let col = image::Rgba([neon_color[0], neon_color[1], neon_color[2], alpha]);
                out.put_pixel(x, y, col);
                continue;
            }

            // Inside 3D crystal glass border
            if dist > 0.0 {
                let u = (dist / border_thickness).clamp(0.0, 1.0);
                let tube_h = (1.0 - (2.0 * u - 1.0).powi(2)).max(0.0).sqrt();

                // Directional 3D light from top-left (-0.6, -0.8)
                let nx = px / (cx.max(1.0));
                let ny = py / (cy.max(1.0));
                let light_dot = (-0.60 * nx - 0.80 * ny).clamp(-1.0, 1.0);

                let spec = (light_dot.max(0.0).powf(1.6) * tube_h * 2.2).clamp(0.0, 1.0);
                let shadow = (-light_dot).max(0.0).powf(1.2) * 0.45;

                let outer_rim = ((u - 0.75) / 0.25).clamp(0.0, 1.0);
                let inner_rim = ((0.25 - u) / 0.25).clamp(0.0, 1.0);

                let base_r = (neon_color[0] as f32 * 0.70) + 65.0;
                let base_g = (neon_color[1] as f32 * 0.70) + 75.0;
                let base_b = (neon_color[2] as f32 * 0.70) + 95.0;

                let mut r = base_r + (255.0 - base_r) * spec + inner_rim * 90.0 + outer_rim * (neon_color[0] as f32 * 0.3);
                let mut g = base_g + (255.0 - base_g) * spec + inner_rim * 90.0 + outer_rim * (neon_color[1] as f32 * 0.3);
                let mut b = base_b + (255.0 - base_b) * spec + inner_rim * 110.0 + outer_rim * (neon_color[2] as f32 * 0.3);

                r = (r * (1.0 - shadow)).clamp(0.0, 255.0);
                g = (g * (1.0 - shadow)).clamp(0.0, 255.0);
                b = (b * (1.0 - shadow)).clamp(0.0, 255.0);

                let alpha = if is_active {
                    (200.0 + 55.0 * tube_h).min(255.0) as u8
                } else {
                    (155.0 + 75.0 * (inner_rim + outer_rim) + 25.0 * tube_h).clamp(0.0, 240.0) as u8
                };

                out.put_pixel(x, y, image::Rgba([r as u8, g as u8, b as u8, alpha]));
                continue;
            }

            // Inside content area (artwork)
            let inner_aa = if dist > -1.0 {
                ((1.0 - dist) * 0.5).clamp(0.0, 1.0)
            } else {
                1.0
            };

            let src_px = *scaled_src.get_pixel(x, y);

            // Curved 3D glass dome reflection across upper hemisphere
            let gy = (py + cy * 0.35) / (card_h * 0.45);
            let gx = px / (card_w * 0.42);
            let g_dist = (gx * gx + gy * gy).sqrt();
            let dome_glare = if py < 15.0 {
                ((1.0 - g_dist).clamp(0.0, 1.0)).powf(2.0) * 0.30
            } else {
                0.0
            };

            let pixel = if is_active {
                // Active item: darkened artwork with rich contrast making glowing crystal frame pop
                let darken = 0.42f32;
                let r = ((src_px[0] as f32 * darken + 255.0 * dome_glare).min(255.0) * inner_aa) as u8;
                let g = ((src_px[1] as f32 * darken + 255.0 * dome_glare).min(255.0) * inner_aa) as u8;
                let b = ((src_px[2] as f32 * darken + 255.0 * dome_glare).min(255.0) * inner_aa) as u8;
                let a = (src_px[3] as f32 * inner_aa) as u8;
                image::Rgba([r, g, b, a])
            } else {
                // Inactive items: crisp natural artwork with glossy crystal dome reflection
                let r = ((src_px[0] as f32 + 255.0 * dome_glare).min(255.0) * inner_aa) as u8;
                let g = ((src_px[1] as f32 + 255.0 * dome_glare).min(255.0) * inner_aa) as u8;
                let b = ((src_px[2] as f32 + 255.0 * dome_glare).min(255.0) * inner_aa) as u8;
                let a = (src_px[3] as f32 * inner_aa) as u8;
                image::Rgba([r, g, b, a])
            };

            out.put_pixel(x, y, pixel);
        }
    }

    out
}

pub fn generate_honeycomb_thumb_sync(
    media_path: &Path,
    is_active: bool,
    accent_rgb: Option<[u8; 3]>,
) -> Option<PathBuf> {
    let cache_dir = get_honeycomb_cache_dir();
    let _ = std::fs::create_dir_all(&cache_dir);
    let cached_path = honeycomb_thumb_path(media_path, is_active, accent_rgb);

    if cached_path.exists() {
        return Some(cached_path);
    }

    let std_thumb = thumb_path_for_video(media_path);
    let src_img_path = if std_thumb.exists() {
        std_thumb
    } else if media_path.exists() {
        media_path.to_path_buf()
    } else {
        return None;
    };

    let base_img = image::open(&src_img_path).ok()?;
    let out = apply_glass_honeycomb_mask(&base_img, is_active, accent_rgb);
    let total_w = out.width();
    let total_h = out.height();

    // Direct GPU/memory cache insertion
    let accent = accent_rgb.unwrap_or([58, 142, 230]);
    let handle_key = format!("hc:{}:{}:{:02x}{:02x}{:02x}", media_path.to_string_lossy(), is_active, accent[0], accent[1], accent[2]);
    let direct_handle = cosmic::iced::widget::image::Handle::from_rgba(total_w, total_h, out.as_raw().clone());
    if let Ok(mut cache) = THUMB_HANDLE_CACHE.lock() {
        cache.insert(handle_key, direct_handle);
    }

    let file = std::fs::File::create(&cached_path).ok()?;
    let writer = std::io::BufWriter::new(file);
    let encoder = PngEncoder::new_with_quality(
        writer,
        CompressionType::Fast,
        FilterType::NoFilter,
    );
    if encoder.write_image(
        out.as_raw(),
        total_w,
        total_h,
        image::ExtendedColorType::Rgba8,
    ).is_ok() {
        Some(cached_path)
    } else {
        None
    }
}

pub const HONEYCOMB_ANIM_FRAME_COUNT: usize = 6;

pub static ANIM_HANDLE_CACHE: LazyLock<Mutex<LruMemoryCache<String, cosmic::iced::widget::image::Handle>>> =
    LazyLock::new(|| Mutex::new(LruMemoryCache::new(256)));

static HONEYCOMB_ANIM_IN_FLIGHT: LazyLock<Mutex<HashSet<PathBuf>>> = LazyLock::new(|| Mutex::new(HashSet::new()));

pub fn get_honeycomb_anim_cache_dir() -> PathBuf {
    let base = if let Some(xdg) = std::env::var_os("XDG_CACHE_HOME") {
        if !xdg.is_empty() {
            PathBuf::from(xdg)
        } else {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
            PathBuf::from(home).join(".cache")
        }
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        PathBuf::from(home).join(".cache")
    };
    base.join("aura/honeycomb_anim")
}

pub fn honeycomb_anim_frame_path(
    media_path: &Path,
    is_active: bool,
    accent_rgb: Option<[u8; 3]>,
    frame_idx: usize,
) -> PathBuf {
    let cache_dir = get_honeycomb_anim_cache_dir();
    let hash = format!("{:016x}", md5_simple(media_path.to_string_lossy().as_bytes()));
    let accent = accent_rgb.unwrap_or([58, 142, 230]);
    let suffix = if is_active {
        format!("act_{:02x}{:02x}{:02x}", accent[0], accent[1], accent[2])
    } else {
        "inact".to_string()
    };
    cache_dir.join(format!("gframe4_{}_{}_{}.png", &hash[..10], suffix, frame_idx % HONEYCOMB_ANIM_FRAME_COUNT))
}

pub fn honeycomb_anim_handle(
    media_path: &Path,
    is_active: bool,
    accent_rgb: Option<[u8; 3]>,
    frame_idx: usize,
) -> Option<cosmic::iced::widget::image::Handle> {
    let accent = accent_rgb.unwrap_or([58, 142, 230]);
    let idx = frame_idx % HONEYCOMB_ANIM_FRAME_COUNT;
    let key = format!("hca:{}:{}:{:02x}{:02x}{:02x}:{}", media_path.to_string_lossy(), is_active, accent[0], accent[1], accent[2], idx);

    if let Ok(mut cache) = ANIM_HANDLE_CACHE.lock() {
        if let Some(handle) = cache.get(&key) {
            return Some(handle);
        }
    }

    let frame_file = honeycomb_anim_frame_path(media_path, is_active, accent_rgb, idx);
    if frame_file.exists() {
        let handle = cosmic::iced::widget::image::Handle::from_path(frame_file);
        if let Ok(mut cache) = ANIM_HANDLE_CACHE.lock() {
            cache.insert(key, handle.clone());
        }
        return Some(handle);
    }

    if is_anim_failed(media_path) {
        return honeycomb_thumb_handle(media_path, is_active, accent_rgb);
    }

    let path_buf = media_path.to_path_buf();
    let mut in_flight = match HONEYCOMB_ANIM_IN_FLIGHT.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    if in_flight.insert(path_buf.clone()) {
        drop(in_flight);
        spawn_bg_task(move || {
            generate_honeycomb_anim_sync(&path_buf, accent_rgb);
            if let Ok(mut guard) = HONEYCOMB_ANIM_IN_FLIGHT.lock() {
                guard.remove(&path_buf);
            }
        });
    }

    honeycomb_thumb_handle(media_path, is_active, accent_rgb)
}

pub fn generate_honeycomb_anim_sync(media_path: &Path, accent_rgb: Option<[u8; 3]>) {
    if !media_path.is_file() || is_anim_failed(media_path) {
        return;
    }
    let ext = media_path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    if ext == "png" || ext == "jpg" || ext == "jpeg" || ext == "webp" {
        return;
    }

    let cache_dir = get_honeycomb_anim_cache_dir();
    let _ = std::fs::create_dir_all(&cache_dir);

    let mut all_exist = true;
    for i in 0..HONEYCOMB_ANIM_FRAME_COUNT {
        let p_inact = honeycomb_anim_frame_path(media_path, false, accent_rgb, i);
        let p_act = honeycomb_anim_frame_path(media_path, true, accent_rgb, i);
        if !p_inact.exists() || !p_act.exists() {
            all_exist = false;
            break;
        }
    }
    if all_exist {
        return;
    }

    let _permit = FfmpegPermit::acquire();

    let hash = format!("{:016x}", md5_simple(media_path.to_string_lossy().as_bytes()));
    let temp_dir = std::env::temp_dir().join(format!("aura_anim_{}_{}", std::process::id(), &hash[..8]));
    let _ = std::fs::create_dir_all(&temp_dir);

    let pattern = temp_dir.join("f_%02d.png");
    let mut cmd = std::process::Command::new("ffmpeg");
    cmd.arg("-nostdin")
        .arg("-y")
        .arg("-threads")
        .arg("1")
        .arg("-ss")
        .arg("00:00:01")
        .arg("-i")
        .arg(media_path)
        .arg("-t")
        .arg("1.6")
        .arg("-r")
        .arg("4")
        .arg("-vf")
        .arg("scale=260:300:force_original_aspect_ratio=increase,crop=260:300")
        .arg(&pattern)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    let _ = cmd.status();

    let mut any_saved = false;
    for i in 0..HONEYCOMB_ANIM_FRAME_COUNT {
        let f_path = temp_dir.join(format!("f_{:02}.png", i + 1));
        let img_opt = if f_path.exists() {
            image::open(&f_path).ok()
        } else {
            let f1 = temp_dir.join("f_01.png");
            if f1.exists() {
                image::open(&f1).ok()
            } else {
                None
            }
        };

        if let Some(img) = img_opt {
            let out_inact = apply_glass_honeycomb_mask(&img, false, accent_rgb);
            let p_inact = honeycomb_anim_frame_path(media_path, false, accent_rgb, i);
            let _ = out_inact.save(&p_inact);

            let out_act = apply_glass_honeycomb_mask(&img, true, accent_rgb);
            let p_act = honeycomb_anim_frame_path(media_path, true, accent_rgb, i);
            let _ = out_act.save(&p_act);
            any_saved = true;
        }
    }
    if !any_saved {
        mark_anim_failed(media_path);
    }

    let _ = std::fs::remove_dir_all(&temp_dir);
}

pub fn honeycomb_thumb_handle(
    media_path: &Path,
    is_active: bool,
    accent_rgb: Option<[u8; 3]>,
) -> Option<cosmic::iced::widget::image::Handle> {
    let accent = accent_rgb.unwrap_or([58, 142, 230]);
    let key = format!("hc:{}:{}:{:02x}{:02x}{:02x}", media_path.to_string_lossy(), is_active, accent[0], accent[1], accent[2]);

    if let Ok(mut cache) = THUMB_HANDLE_CACHE.lock() {
        if let Some(handle) = cache.get(&key) {
            return Some(handle);
        }
    }

    let cached_path = honeycomb_thumb_path(media_path, is_active, accent_rgb);
    if cached_path.exists() {
        let handle = cosmic::iced::widget::image::Handle::from_path(cached_path);
        if let Ok(mut cache) = THUMB_HANDLE_CACHE.lock() {
            cache.insert(key, handle.clone());
        }
        return Some(handle);
    }

    if tokio::runtime::Handle::try_current().is_ok() {
        trigger_honeycomb_thumb_async(media_path.to_path_buf(), is_active, accent_rgb);
        None
    } else {
        let path = generate_honeycomb_thumb_sync(media_path, is_active, accent_rgb)?;
        let handle = cosmic::iced::widget::image::Handle::from_path(path);
        if let Ok(mut cache) = THUMB_HANDLE_CACHE.lock() {
            cache.insert(key, handle.clone());
        }
        Some(handle)
    }
}

pub fn trigger_honeycomb_thumb_async(
    media_path: PathBuf,
    is_active: bool,
    accent_rgb: Option<[u8; 3]>,
) {
    let cached_path = honeycomb_thumb_path(&media_path, is_active, accent_rgb);
    if cached_path.exists() {
        return;
    }

    let mut in_flight = match HONEYCOMB_IN_FLIGHT.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    if in_flight.insert(cached_path.clone()) {
        drop(in_flight);
        spawn_bg_task(move || {
            let _ = generate_honeycomb_thumb_sync(&media_path, is_active, accent_rgb);
            if let Ok(mut in_flight) = HONEYCOMB_IN_FLIGHT.lock() {
                in_flight.remove(&cached_path);
            }
        });
    }
}

pub fn prewarm_honeycomb_thumbs(media_paths: &[PathBuf], accent_rgb: [u8; 3]) {
    for path in media_paths {
        trigger_honeycomb_thumb_async(path.clone(), true, Some(accent_rgb));
        trigger_honeycomb_thumb_async(path.clone(), false, Some(accent_rgb));
        if is_anim_failed(path) || honeycomb_anim_frame_path(path, false, Some(accent_rgb), 0).exists() {
            continue;
        }
        let p = path.clone();
        let mut in_flight = match HONEYCOMB_ANIM_IN_FLIGHT.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        if in_flight.insert(p.clone()) {
            drop(in_flight);
            spawn_bg_task(move || {
                generate_honeycomb_anim_sync(&p, Some(accent_rgb));
                if let Ok(mut guard) = HONEYCOMB_ANIM_IN_FLIGHT.lock() {
                    guard.remove(&p);
                }
            });
        }
    }
}

pub fn honeycomb_thumb_for_media(
    media_path: &Path,
    is_active: bool,
    accent_rgb: Option<[u8; 3]>,
) -> Option<PathBuf> {
    let cached_path = honeycomb_thumb_path(media_path, is_active, accent_rgb);
    if cached_path.exists() {
        return Some(cached_path);
    }

    if tokio::runtime::Handle::try_current().is_ok() {
        trigger_honeycomb_thumb_async(media_path.to_path_buf(), is_active, accent_rgb);
        None
    } else {
        generate_honeycomb_thumb_sync(media_path, is_active, accent_rgb)
    }
}

pub fn optimize_wallpaper_image(image_path: &Path, max_w: u32, max_h: u32) -> PathBuf {
    let (orig_w, orig_h) = match image::image_dimensions(image_path) {
        Ok(dims) => dims,
        Err(_) => return image_path.to_path_buf(),
    };

    if orig_w <= max_w && orig_h <= max_h {
        return image_path.to_path_buf();
    }

    let cache_dir = get_scaled_cache_dir();
    let _ = std::fs::create_dir_all(&cache_dir);

    let file_stem = image_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "img".into());
    let clean_stem: String = file_stem
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .take(50)
        .collect();
    let hash = format!("{:016x}", md5_simple(image_path.to_string_lossy().as_bytes()));

    let mtime = std::fs::metadata(image_path)
        .and_then(|m| m.modified())
        .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs())
        .unwrap_or(0);

    let is_png = image_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("png"))
        .unwrap_or(false);
    let ext = if is_png { "png" } else { "jpg" };

    let cached_path = cache_dir.join(format!("{}_{}_{}_{}x{}.{}", clean_stem, &hash[..8], mtime, max_w, max_h, ext));
    if cached_path.exists() {
        return cached_path;
    }

    match image::open(image_path) {
        Ok(img) => {
            let scaled = img.resize(max_w, max_h, image::imageops::FilterType::Lanczos3);
            if is_png {
                if scaled.save_with_format(&cached_path, image::ImageFormat::Png).is_ok() {
                    return cached_path;
                }
            } else if let Ok(mut file) = std::fs::File::create(&cached_path) {
                let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut file, 95);
                if encoder.encode_image(&scaled).is_ok() {
                    return cached_path;
                }
            }
        }
        Err(e) => {
            eprintln!("[Aura Engine] No se pudo reescalar imagen {}: {}", image_path.display(), e);
        }
    }

    image_path.to_path_buf()
}

pub fn thumb_path_for_video(video_path: &Path) -> PathBuf {
    let cache_dir = get_cache_dir();
    let file_stem = video_path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "video".into());
    // Sanitize name and add hash based on full path to avoid collision
    let clean_stem: String = file_stem.chars().filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-').take(60).collect();
    let hash = format!("{:016x}", md5_simple(video_path.to_string_lossy().as_bytes()));
    cache_dir.join(format!("{}_{}.jpg", clean_stem, &hash[..8]))
}

fn md5_simple(bytes: &[u8]) -> u64 {
    // Fast lightweight FNV-1a hash for deterministic thumb names
    let mut hash = 0xcbf29ce484222325u64;
    for &byte in bytes {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_md5_simple_deterministic() {
        let h1 = md5_simple(b"/home/user/Videos/wallpaper.mp4");
        let h2 = md5_simple(b"/home/user/Videos/wallpaper.mp4");
        let h3 = md5_simple(b"/home/user/Videos/other.mp4");
        assert_eq!(h1, h2);
        assert_ne!(h1, h3);
    }

    #[test]
    fn test_thumb_path_for_video() {
        let p = Path::new("/path/to/my video file!.mp4");
        let thumb = thumb_path_for_video(p);
        let file_name = thumb.file_name().unwrap().to_str().unwrap();
        assert!(file_name.starts_with("myvideofile_"));
        assert!(file_name.ends_with(".jpg"));
        assert!(thumb.starts_with(get_cache_dir()));
    }

    #[test]
    fn test_hash_leading_zeros_safety() {
        let hash = format!("{:016x}", 0u64);
        assert_eq!(hash.len(), 16);
        assert_eq!(&hash[..8], "00000000");
    }

    #[test]
    fn test_cinematic_thumb_generation() {
        let test_dir = std::env::temp_dir().join("aura_test_cinematic");
        let _ = std::fs::create_dir_all(&test_dir);
        let src_img_path = test_dir.join("test_sample.png");

        // Create a dummy 200x200 image
        let dummy = image::RgbaImage::from_pixel(200, 200, image::Rgba([100, 150, 200, 255]));
        dummy.save_with_format(&src_img_path, image::ImageFormat::Png).expect("save dummy");

        let generated = cinematic_thumb_for_media(&src_img_path, true, Some([255, 100, 50]));
        assert!(generated.is_some(), "Cinematic thumbnail should be generated");
        let path = generated.unwrap();
        assert!(path.to_string_lossy().ends_with("_act_ff6432.png"));

        // Verify image dimensions and transparency in skewed corner
        let img = image::open(&path).expect("open generated thumb").to_rgba8();
        assert_eq!(img.width(), 481);
        assert_eq!(img.height(), 440);
        // Top-left pixel (0, 0) should be outside the tilted parallelogram (alpha == 0)
        assert_eq!(img.get_pixel(0, 0)[3], 0);

        // Clean up
        let _ = std::fs::remove_file(&src_img_path);
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_cinematic_anim_paths() {
        let p = Path::new("/path/to/sample.mp4");
        let frame0 = cinematic_anim_frame_path(p, 0);
        let frame29 = cinematic_anim_frame_path(p, 29);
        let frame30 = cinematic_anim_frame_path(p, 30);
        assert!(frame0.to_string_lossy().ends_with("_0.jpg"));
        assert!(frame29.to_string_lossy().ends_with("_29.jpg"));
        assert!(frame30.to_string_lossy().ends_with("_0.jpg"));
        assert!(frame0.starts_with(get_cinematic_anim_cache_dir()));
    }

    #[test]
    fn test_honeycomb_thumb_generation() {
        let test_dir = std::env::temp_dir().join("aura_test_honeycomb");
        let _ = std::fs::create_dir_all(&test_dir);
        let src_img_path = test_dir.join("test_hex_sample.png");

        let dummy = image::RgbaImage::from_pixel(200, 200, image::Rgba([120, 180, 240, 255]));
        dummy.save_with_format(&src_img_path, image::ImageFormat::Png).expect("save dummy hex");

        let generated = honeycomb_thumb_for_media(&src_img_path, true, Some([255, 100, 50]));
        assert!(generated.is_some(), "Honeycomb thumbnail should be generated");
        let path = generated.unwrap();
        assert!(path.to_string_lossy().ends_with("_act_ff6432.png"));

        let img = image::open(&path).expect("open generated hex thumb").to_rgba8();
        assert_eq!(img.width(), 260);
        assert_eq!(img.height(), 300);
        // Corner (0, 0) of pointy-topped hexagon must be transparent (alpha == 0)
        assert_eq!(img.get_pixel(0, 0)[3], 0);
        // Center pixel should be opaque (alpha == 255)
        assert_eq!(img.get_pixel(130, 150)[3], 255);

        // Clean up
        let _ = std::fs::remove_file(&src_img_path);
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_lru_memory_cache() {
        let mut cache = LruMemoryCache::<String, usize>::new(3);
        cache.insert("a".into(), 1);
        cache.insert("b".into(), 2);
        cache.insert("c".into(), 3);
        assert_eq!(cache.get(&"a".into()), Some(1));
        // Inserting "d" should evict "b" since "a" was recently accessed
        cache.insert("d".into(), 4);
        assert_eq!(cache.get(&"b".into()), None);
        assert_eq!(cache.get(&"a".into()), Some(1));
        assert_eq!(cache.get(&"c".into()), Some(3));
        assert_eq!(cache.get(&"d".into()), Some(4));
    }
}

static THUMB_SEMAPHORE: std::sync::OnceLock<tokio::sync::Semaphore> = std::sync::OnceLock::new();

pub async fn generate_thumbnail(media_path: &Path) -> Option<PathBuf> {
    let thumb_path = thumb_path_for_video(media_path);
    if thumb_path.exists() {
        return Some(thumb_path);
    }

    let sem = THUMB_SEMAPHORE.get_or_init(|| tokio::sync::Semaphore::new(2));
    let _permit = sem.acquire().await.ok()?;

    if thumb_path.exists() {
        return Some(thumb_path);
    }

    if let Some(parent) = thumb_path.parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }

    let is_image = media_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|ext| crate::scanner::IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false);

    if is_image {
        let src = media_path.to_path_buf();
        let dst = thumb_path.clone();
        let res = tokio::task::spawn_blocking(move || {
            if let Ok(img) = image::open(&src) {
                let thumb = img.thumbnail(480, 270);
                if thumb.save(&dst).is_ok() && dst.exists() {
                    return Some(dst);
                }
            }
            None
        }).await.ok().flatten();

        if res.is_some() {
            return res;
        }
    }

    let _permit = ASYNC_FFMPEG_SEM.acquire().await.ok();
    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-nostdin")
        .arg("-y")
        .arg("-threads")
        .arg("1")
        .arg("-ss")
        .arg("00:00:01")
        .arg("-i")
        .arg(media_path)
        .arg("-an")
        .arg("-sn")
        .arg("-vframes")
        .arg("1")
        .arg("-vf")
        .arg("scale=480:-1")
        .arg(&thumb_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    let status = cmd.status().await;

    match status {
        Ok(s) if s.success() && thumb_path.exists() => Some(thumb_path),
        _ => None,
    }
}
