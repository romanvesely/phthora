//! Built-in resources (LUTs, fonts, overlay videos), user images, and video sources.

use crate::context::{GpuContext, Target};
use crate::text::FontData;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// A decoded video that can be sampled at arbitrary times.
pub trait VideoSource {
    fn size(&self) -> (u32, u32);
    fn duration(&self) -> f64;
    /// Frame for time `t` in seconds (wraps at the duration): frame index and
    /// top-down, straight-alpha RGBA rows of `size()`.
    fn frame_at(&mut self, t: f64) -> Option<(u64, &[u8])>;
}

pub type VideoOpener = Box<dyn Fn(&Path) -> Option<Box<dyn VideoSource>>>;
/// Decodes an image file to (width, height, top-down straight RGBA).
pub type ImageLoader = Box<dyn Fn(&Path) -> Option<(u32, u32, Vec<u8>)>>;

macro_rules! luts {
    ($($n:literal),*) => { &[$(($n, include_bytes!(concat!("../../../assets/lut/", $n, ".png")) as &[u8])),*] };
}

static LUTS: &[(&str, &[u8])] = luts!(
    "amatorka", "brannan", "earlybird", "etikate", "gotham", "hefe", "inkwell", "kelvin", "lofi", "lookup", "nashville",
    "sutro", "toaster", "walden", "xpro"
);

static FONT_SANS: &[u8] = include_bytes!("../../../assets/fonts/roboto-regular.ttf");
static FONT_MONO: &[u8] = include_bytes!("../../../assets/fonts/roboto-mono-regular.ttf");
static FONT_VCR: &[u8] = include_bytes!("../../../assets/fonts/vcr-osd-mono.ttf");
static FONT_PIXEL: &[u8] = include_bytes!("../../../assets/fonts/press-start-2p.ttf");

/// Image file extensions handled as stills (everything else is treated as video).
pub const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp", "bmp"];

pub fn is_image_path(p: &Path) -> bool {
    p.extension().and_then(|e| e.to_str()).map(|e| IMAGE_EXTENSIONS.contains(&e.to_lowercase().as_str())).unwrap_or(false)
}

/// Locate the directory holding `video/` overlay clips.
pub fn find_asset_dir() -> Option<PathBuf> {
    let mut cands: Vec<PathBuf> = Vec::new();
    if let Ok(p) = std::env::var("PHTHORA_ASSETS") {
        cands.push(p.into());
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(d) = exe.parent() {
            cands.push(d.join("assets"));
            cands.push(d.join("../share/phthora/assets"));
            cands.push(d.join("../../assets"));
        }
    cands.push(PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets")));
    cands.into_iter().find(|p| p.join("video").is_dir())
}

/// Decode a PNG to (w, h, top-down straight RGBA).
pub fn decode_png(bytes: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    let mut dec = png::Decoder::new(std::io::Cursor::new(bytes));
    dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = dec.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    let (w, h) = (info.width, info.height);
    let px = (w * h) as usize;
    let src = &buf[..info.buffer_size()];
    let rgba = match info.color_type {
        png::ColorType::Rgba => src.to_vec(),
        png::ColorType::Rgb => src.as_chunks::<3>().0.iter().flat_map(|c| [c[0], c[1], c[2], 255]).collect(),
        png::ColorType::GrayscaleAlpha => src.as_chunks::<2>().0.iter().flat_map(|c| [c[0], c[0], c[0], c[1]]).collect(),
        png::ColorType::Grayscale => src.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::Indexed => return None,
    };
    (rgba.len() == px * 4).then_some((w, h, rgba))
}

/// Create a texture from top-down RGBA. `flip_y` stores the bottom row first,
/// matching how image textures are sampled by the effect shaders.
pub fn texture_from_rgba(ctx: &GpuContext, w: u32, h: u32, data: &[u8], flip_y: bool, label: &str) -> Target {
    let t = Target::new(ctx, w, h, label);
    upload(ctx, &t, data, flip_y);
    t
}

pub fn upload(ctx: &GpuContext, t: &Target, data: &[u8], flip_y: bool) {
    if flip_y {
        let mut d = data.to_vec();
        crate::context::flip_rows(&mut d, t.width, t.height);
        t.upload_rgba8(ctx, &d);
    } else {
        t.upload_rgba8(ctx, data);
    }
}

/// A video clip streamed into a texture.
pub struct VideoTexture {
    src: Option<Box<dyn VideoSource>>,
    pub target: Rc<Target>,
    last: Option<u64>,
}

impl VideoTexture {
    /// `fallback` is used when no video backend is available.
    pub fn open(ctx: &GpuContext, assets: &Assets, path: &Path, fallback: [u8; 4]) -> Self {
        let src = assets.video_opener.as_ref().and_then(|o| o(path));
        let target = match &src {
            Some(s) => {
                let (w, h) = s.size();
                Rc::new(Target::new(ctx, w, h, "video"))
            }
            None => assets.solid(ctx, fallback),
        };
        Self { src, target, last: None }
    }

    pub fn duration(&self) -> f64 {
        self.src.as_ref().map(|s| s.duration()).unwrap_or(0.0)
    }

    /// false if the clip could not be opened (the fallback color shows instead)
    pub fn is_open(&self) -> bool {
        self.src.is_some()
    }

    pub fn size(&self) -> (u32, u32) {
        self.src.as_ref().map(|s| s.size()).unwrap_or((1, 1))
    }

    pub fn update(&mut self, ctx: &GpuContext, t: f64) {
        let Some(src) = self.src.as_mut() else { return };
        if let Some((idx, data)) = src.frame_at(t)
            && self.last != Some(idx) {
                upload(ctx, &self.target, data, true);
                self.last = Some(idx);
            }
    }
}

#[derive(Default)]
pub struct Assets {
    pub dir: Option<PathBuf>,
    pub video_opener: Option<VideoOpener>,
    pub image_loader: Option<ImageLoader>,
    luts: HashMap<String, Rc<Target>>,
    images: HashMap<PathBuf, Option<(Rc<Target>, f32)>>,
    fonts: HashMap<String, Vec<FontData>>,
    solids: std::cell::RefCell<HashMap<[u8; 4], Rc<Target>>>,
    sysdb: Option<fontdb::Database>,
}

impl Assets {
    pub fn new() -> Self {
        Self { dir: find_asset_dir(), ..Default::default() }
    }

    pub fn video_path(&self, name: &str) -> Option<PathBuf> {
        let dir = self.dir.clone().or_else(find_asset_dir)?;
        let p = dir.join("video").join(name);
        p.exists().then_some(p)
    }

    /// 1x1 texture of a single color (straight RGBA).
    pub fn solid(&self, ctx: &GpuContext, c: [u8; 4]) -> Rc<Target> {
        self.solids
            .borrow_mut()
            .entry(c)
            .or_insert_with(|| Rc::new(texture_from_rgba(ctx, 1, 1, &c, false, "solid")))
            .clone()
    }

    pub fn lut(&mut self, ctx: &GpuContext, name: &str) -> Option<Rc<Target>> {
        if let Some(t) = self.luts.get(name) {
            return Some(t.clone());
        }
        let (_, bytes) = LUTS.iter().find(|(n, _)| *n == name)?;
        let (w, h, d) = decode_png(bytes)?;
        let t = Rc::new(texture_from_rgba(ctx, w, h, &d, true, name));
        self.luts.insert(name.to_string(), t.clone());
        Some(t)
    }

    /// Load a user image (cached by path). Returns the texture and its aspect ratio.
    pub fn image(&mut self, ctx: &GpuContext, path: &Path) -> Option<(Rc<Target>, f32)> {
        if let Some(v) = self.images.get(path) {
            return v.clone();
        }
        let decoded = match &self.image_loader {
            Some(l) => l(path),
            None => std::fs::read(path).ok().and_then(|b| decode_png(&b)),
        };
        let v = decoded.map(|(w, h, d)| (Rc::new(texture_from_rgba(ctx, w, h, &d, true, "image")), w as f32 / h.max(1) as f32));
        self.images.insert(path.to_path_buf(), v.clone());
        v
    }

    /// Forget a cached image (after the file changed).
    pub fn forget_image(&mut self, path: &Path) {
        self.images.remove(path);
    }

    fn system_font(&mut self, families: &[&str], bold: bool) -> Option<FontData> {
        let db = self.sysdb.get_or_insert_with(|| {
            let mut db = fontdb::Database::new();
            db.load_system_fonts();
            db
        });
        for fam in families {
            let q = fontdb::Query {
                families: &[fontdb::Family::Name(fam)],
                weight: if bold { fontdb::Weight::BOLD } else { fontdb::Weight::NORMAL },
                ..Default::default()
            };
            if let Some(id) = db.query(&q) {
                // the query falls back to other families; make sure we got the one asked for
                let ok = db.face(id).map(|f| f.families.iter().any(|(n, _)| n.eq_ignore_ascii_case(fam))).unwrap_or(false);
                if !ok {
                    continue;
                }
                if let Some(fd) = db.with_face_data(id, |data, index| FontData { data: std::sync::Arc::new(data.to_vec()), index }) {
                    return Some(fd);
                }
            }
        }
        None
    }

    /// Font chain for a named font (`Sans`, `Serif`, `Mono`, `Impact`, `VCR`, `Pixel`).
    /// Later entries are fallbacks for missing glyphs.
    pub fn font(&mut self, name: &str) -> Vec<FontData> {
        if let Some(f) = self.fonts.get(name) {
            return f.clone();
        }
        let sans = FontData::from_static(FONT_SANS);
        let primary = match name {
            "Mono" => Some(FontData::from_static(FONT_MONO)),
            "VCR" => Some(FontData::from_static(FONT_VCR)),
            "Pixel" => Some(FontData::from_static(FONT_PIXEL)),
            "Serif" => self.system_font(&["Times New Roman", "Times", "Liberation Serif", "Tinos", "Noto Serif", "DejaVu Serif"], false),
            "Impact" => self
                .system_font(&["Impact", "Anton", "Oswald", "League Gothic", "Bebas Neue"], false)
                .or_else(|| self.system_font(&["Liberation Sans Narrow", "DejaVu Sans Condensed", "DejaVu Sans"], true)),
            _ => None,
        };
        let mut chain: Vec<FontData> = primary.into_iter().collect();
        chain.push(sans);
        if let Some(sym) = self.system_font(&["DejaVu Sans", "Noto Sans Symbols2", "Noto Sans Symbols"], false) {
            chain.push(sym);
        }
        self.fonts.insert(name.to_string(), chain.clone());
        chain
    }
}
