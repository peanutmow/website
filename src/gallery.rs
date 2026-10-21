use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::SystemTime;

const PHOTOS: &str = include_str!("../gallery/photos.json");
const CURATION: &str = include_str!("../gallery/curation.json");
const SHEET_SIZE: usize = 36;
const COLOUR_BUCKET: i64 = 30;
const WIDTHS: &[(&str, u32)] = &[
    ("s", 240),
    ("m", 500),
    ("z", 640),
    ("c", 800),
    ("b", 1024),
    ("l", 1600),
    ("h", 2048),
];
const GALLERY_SIZES: &str = "(max-width: 720px) 45vw, (max-width: 1100px) 30vw, 230px";

#[derive(Deserialize, Default)]
struct Curation {
    #[serde(default)]
    photos: HashMap<String, Curated>,
}

#[derive(Deserialize, Default)]
struct Curated {
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    r#type: Option<String>,
    #[serde(default)]
    caption: Option<String>,
    #[serde(default)]
    alt: Option<String>,
    #[serde(default)]
    exif: Option<Map<String, Value>>,
}

#[derive(Deserialize, Clone, Default)]
struct Sizes {
    #[serde(default)]
    s: Option<String>,
    #[serde(default)]
    m: Option<String>,
    #[serde(default)]
    z: Option<String>,
    #[serde(default)]
    c: Option<String>,
    #[serde(default)]
    b: Option<String>,
    #[serde(default)]
    l: Option<String>,
    #[serde(default)]
    h: Option<String>,
}

impl Sizes {
    fn get(&self, key: &str) -> Option<&str> {
        match key {
            "s" => self.s.as_deref(),
            "m" => self.m.as_deref(),
            "z" => self.z.as_deref(),
            "c" => self.c.as_deref(),
            "b" => self.b.as_deref(),
            "l" => self.l.as_deref(),
            "h" => self.h.as_deref(),
            _ => None,
        }
    }
}

#[derive(Deserialize, Clone)]
struct Photo {
    id: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    width: Option<u32>,
    #[serde(default)]
    height: Option<u32>,
    #[serde(default)]
    sizes: Sizes,
    #[serde(default)]
    feed_tags: Vec<String>,
    #[serde(default)]
    palette: Vec<String>,
    #[serde(default)]
    hue: Option<f64>,
    #[serde(default)]
    sat: Option<f64>,
    #[serde(rename = "type", default)]
    kind: Option<String>,
    #[serde(default)]
    caption: Option<String>,
    #[serde(default)]
    alt: Option<String>,
    #[serde(default)]
    exif: Option<Map<String, Value>>,
    #[serde(default)]
    url: Option<String>,
    #[serde(skip)]
    tags: Vec<String>,
}

#[derive(Deserialize)]
struct Dataset {
    #[serde(default)]
    owner: Option<String>,
    #[serde(default)]
    generated: Option<String>,
    #[serde(default)]
    photos: Vec<Photo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Swatch {
    pub hue: i64,
    pub count: usize,
    pub colour: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TagCount {
    pub name: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Frame {
    pub id: String,
    pub number: String,
    pub alt: String,
    pub label: String,
    pub src: String,
    pub srcset: String,
    pub best: String,
    pub loading: &'static str,
    pub eager: bool,
    pub sizes: &'static str,
    pub hue: Option<i64>,
    pub sat: f64,
    pub tags_json: String,
    pub meta: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Sheet {
    pub number: String,
    pub start: String,
    pub end: String,
    pub total: usize,
    pub frames: Vec<Frame>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Gallery {
    pub owner: String,
    pub generated: Option<String>,
    pub total: usize,
    pub sheet_count: usize,
    pub mono_count: usize,
    pub untagged: usize,
    pub swatches: Vec<Swatch>,
    pub tags: Vec<TagCount>,
    pub sheets: Vec<Sheet>,
    pub photo_order: Vec<String>,
}

type Cache = Option<(SystemTime, SystemTime, Arc<Gallery>)>;

pub fn data() -> Arc<Gallery> {
    static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(None));

    let photos_mtime = mtime("gallery/photos.json");
    let curation_mtime = mtime("gallery/curation.json");

    let mut guard = cache.lock().unwrap_or_else(|error| error.into_inner());
    if let Some((cached_photos, cached_curation, gallery)) = guard.as_ref() {
        if *cached_photos == photos_mtime && *cached_curation == curation_mtime {
            return Arc::clone(gallery);
        }
    }

    let photos = fs::read_to_string("gallery/photos.json").unwrap_or_else(|_| PHOTOS.to_string());
    let curation =
        fs::read_to_string("gallery/curation.json").unwrap_or_else(|_| CURATION.to_string());
    let gallery = Arc::new(build(&photos, &curation));
    *guard = Some((photos_mtime, curation_mtime, Arc::clone(&gallery)));
    gallery
}

fn mtime(path: &str) -> SystemTime {
    fs::metadata(Path::new(path))
        .and_then(|meta| meta.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH)
}

pub fn count() -> usize {
    data().total
}

fn build(photos: &str, curation: &str) -> Gallery {
    let mut dataset: Dataset = serde_json::from_str(photos).unwrap_or(Dataset {
        owner: None,
        generated: None,
        photos: Vec::new(),
    });
    let curated: Curation = serde_json::from_str(curation).unwrap_or_default();

    for photo in &mut dataset.photos {
        if let Some(entry) = curated.photos.get(&photo.id) {
            if !entry.tags.is_empty() {
                photo.tags = entry.tags.clone();
            }
            if let Some(kind) = &entry.r#type {
                photo.kind = Some(kind.clone());
            }
            if let Some(caption) = &entry.caption {
                photo.caption = Some(caption.clone());
            }
            if let Some(alt) = &entry.alt {
                photo.alt = Some(alt.clone());
            }
            if let Some(exif) = &entry.exif {
                photo.exif = Some(exif.clone());
            }
        }
        if photo.tags.is_empty() {
            photo.tags = photo.feed_tags.clone();
        }
        if photo.kind.is_none() {
            photo.kind = Some("photography".to_string());
        }
    }

    let owner = dataset.owner.clone().unwrap_or_default();
    let total = dataset.photos.len();
    let sheet_count = total.div_ceil(SHEET_SIZE);
    let mono_count = dataset
        .photos
        .iter()
        .filter(|photo| photo.sat.unwrap_or(1.0) < 0.15)
        .count();
    let untagged = dataset
        .photos
        .iter()
        .filter(|photo| photo.tags.is_empty())
        .count();

    let swatches = build_swatches(&dataset.photos);
    let tags = build_tags(&dataset.photos);
    let photo_order = dataset
        .photos
        .iter()
        .map(|photo| photo.id.clone())
        .collect();

    let mut sheets = Vec::with_capacity(sheet_count);
    for (sheet_index, slice) in dataset.photos.chunks(SHEET_SIZE).enumerate() {
        let start_number = sheet_index * SHEET_SIZE + 1;
        let end_number = start_number + slice.len() - 1;
        let frames = slice
            .iter()
            .enumerate()
            .map(|(index, photo)| {
                let frame_number = start_number + index;
                frame(photo, frame_number, &owner)
            })
            .collect();
        sheets.push(Sheet {
            number: format!("{:02}", sheet_index + 1),
            start: format!("{start_number:03}"),
            end: format!("{end_number:03}"),
            total: slice.len(),
            frames,
        });
    }

    Gallery {
        owner,
        generated: dataset.generated,
        total,
        sheet_count,
        mono_count,
        untagged,
        swatches,
        tags,
        sheets,
        photo_order,
    }
}

fn frame(photo: &Photo, frame_number: usize, owner: &str) -> Frame {
    let label = primary_label(photo);
    let alt = alt_text(photo);
    let src = photo
        .sizes
        .get("z")
        .or_else(|| photo.sizes.get("c"))
        .or_else(|| photo.sizes.get("m"))
        .or_else(|| photo.sizes.get("s"))
        .unwrap_or("")
        .to_string();
    let best = best_image(photo);
    let srcset = srcset(photo);
    let tags_json = serde_json::to_string(&photo.tags).unwrap_or_else(|_| "[]".to_string());
    let meta = json!({
        "title": &photo.title,
        "width": photo.width,
        "height": photo.height,
        "type": &photo.kind,
        "caption": &photo.caption,
        "palette": &photo.palette,
        "exif": &photo.exif,
        "url": photo.url.clone().unwrap_or_else(|| format!("https://www.flickr.com/photos/{owner}/{}/", photo.id)),
        "srcset": &srcset,
        "best": &best,
    })
    .to_string();

    Frame {
        id: photo.id.clone(),
        number: format!("{frame_number:03}"),
        alt,
        label,
        src,
        srcset,
        best,
        loading: if frame_number <= 6 { "eager" } else { "lazy" },
        eager: frame_number <= 6,
        sizes: GALLERY_SIZES,
        hue: photo
            .hue
            .filter(|_| photo.sat.unwrap_or(0.0) >= 0.12)
            .map(|hue| (hue / COLOUR_BUCKET as f64).floor() as i64 * COLOUR_BUCKET),
        sat: photo.sat.unwrap_or(1.0),
        tags_json,
        meta,
    }
}

fn build_swatches(photos: &[Photo]) -> Vec<Swatch> {
    let mut buckets: BTreeMap<i64, (usize, f64, f64, f64, usize)> = BTreeMap::new();
    for photo in photos {
        let Some(hue) = photo.hue else { continue };
        if photo.sat.unwrap_or(0.0) < 0.12 {
            continue;
        }
        let bucket = (hue / COLOUR_BUCKET as f64).floor() as i64 * COLOUR_BUCKET;
        let entry = buckets.entry(bucket).or_insert((0, 0.0, 0.0, 0.0, 0));
        entry.0 += 1;
        if let Some([r, g, b]) = most_saturated(&photo.palette) {
            entry.1 += f64::from(r);
            entry.2 += f64::from(g);
            entry.3 += f64::from(b);
            entry.4 += 1;
        }
    }

    buckets
        .into_iter()
        .map(|(bucket, (count, r, g, b, sampled))| {
            let centre = bucket as f64 + COLOUR_BUCKET as f64 / 2.0;
            let mut colour = format!("hsl({centre} 32% 42%)");
            if sampled > 0 {
                let r = (r / sampled as f64).round() as u32;
                let g = (g / sampled as f64).round() as u32;
                let b = (b / sampled as f64).round() as u32;
                let lum =
                    (0.2126 * f64::from(r) + 0.7152 * f64::from(g) + 0.0722 * f64::from(b)) / 255.0;
                if (0.16..=0.82).contains(&lum) {
                    colour = format!("rgb({r} {g} {b})");
                }
            }
            Swatch {
                hue: bucket,
                count,
                colour,
                name: hue_name(centre).to_string(),
            }
        })
        .collect()
}

fn build_tags(photos: &[Photo]) -> Vec<TagCount> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for photo in photos {
        for tag in &photo.tags {
            *counts.entry(tag.clone()).or_insert(0) += 1;
        }
    }
    let mut tags: Vec<TagCount> = counts
        .into_iter()
        .map(|(name, count)| TagCount { name, count })
        .collect();
    tags.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    tags
}

fn srcset(photo: &Photo) -> String {
    WIDTHS
        .iter()
        .filter_map(|(key, width)| photo.sizes.get(key).map(|url| format!("{url} {width}w")))
        .collect::<Vec<_>>()
        .join(", ")
}

fn best_image(photo: &Photo) -> String {
    ["l", "b", "c", "z", "m"]
        .iter()
        .find_map(|key| photo.sizes.get(key))
        .unwrap_or("")
        .to_string()
}

fn primary_label(photo: &Photo) -> String {
    if !photo.tags.is_empty() {
        return photo
            .tags
            .iter()
            .take(2)
            .cloned()
            .collect::<Vec<_>>()
            .join(" · ");
    }
    if let Some(caption) = &photo.caption {
        if !caption.is_empty() {
            return caption.clone();
        }
    }
    if let Some(title) = &photo.title {
        if !title.is_empty() && !is_camera_name(title) {
            return title.clone();
        }
    }
    "untitled".to_string()
}

fn alt_text(photo: &Photo) -> String {
    if let Some(alt) = &photo.alt {
        if !alt.is_empty() {
            return alt.clone();
        }
    }
    if let Some(caption) = &photo.caption {
        if !caption.is_empty() {
            return caption.clone();
        }
    }
    if !photo.tags.is_empty() {
        return format!("Photograph — {}", photo.tags.join(", "));
    }
    "Photograph".to_string()
}

fn is_camera_name(title: &str) -> bool {
    let upper = title.to_uppercase();
    let trimmed = upper.strip_prefix('_').unwrap_or(&upper);
    for prefix in ["DSCN", "DSC", "IMG", "PXL", "GOPR", "DJI", "MVI", "VID"] {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            let rest = rest
                .strip_prefix('-')
                .or_else(|| rest.strip_prefix('_'))
                .unwrap_or(rest);
            if rest.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
                return true;
            }
        }
    }
    let all_digits = title.len() >= 6 && title.chars().all(|ch| ch.is_ascii_digit());
    all_digits || title.eq_ignore_ascii_case("photo")
}

fn hex_to_rgb(hex: &str) -> Option<[u32; 3]> {
    let hex = hex.strip_prefix('#').unwrap_or(hex);
    if hex.len() != 6 || !hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return None;
    }
    let r = u32::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u32::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u32::from_str_radix(&hex[4..6], 16).ok()?;
    Some([r, g, b])
}

fn most_saturated(palette: &[String]) -> Option<[u32; 3]> {
    let mut best: Option<([u32; 3], u32)> = None;
    for colour in palette {
        let Some(rgb) = hex_to_rgb(colour) else {
            continue;
        };
        let score = rgb[0].max(rgb[1]).max(rgb[2]) - rgb[0].min(rgb[1]).min(rgb[2]);
        if best.is_none_or(|(_, current)| score > current) {
            best = Some((rgb, score));
        }
    }
    best.map(|(rgb, _)| rgb)
}

fn hue_name(hue: f64) -> &'static str {
    const NAMES: &[(f64, &str)] = &[
        (20.0, "red"),
        (40.0, "rust"),
        (60.0, "amber"),
        (80.0, "olive"),
        (110.0, "moss"),
        (150.0, "green"),
        (180.0, "pine"),
        (200.0, "teal"),
        (225.0, "cyan"),
        (250.0, "steel"),
        (275.0, "indigo"),
        (305.0, "violet"),
        (330.0, "magenta"),
        (350.0, "rose"),
        (361.0, "red"),
    ];
    NAMES
        .iter()
        .find(|(limit, _)| hue < *limit)
        .map(|(_, name)| *name)
        .unwrap_or("red")
}

#[cfg(test)]
pub fn filter_counts(gallery: &Gallery, hue: Option<i64>, mono: bool, tag: Option<&str>) -> usize {
    gallery
        .sheets
        .iter()
        .flat_map(|sheet| &sheet.frames)
        .filter(|frame| matches_filter(frame, hue, mono, tag))
        .count()
}

#[cfg(test)]
fn matches_filter(frame: &Frame, hue: Option<i64>, mono: bool, tag: Option<&str>) -> bool {
    let tags: Vec<String> = serde_json::from_str(&frame.tags_json).unwrap_or_default();
    if let Some(tag) = tag {
        if tag == "__untagged" {
            if !tags.is_empty() {
                return false;
            }
        } else if !tags.iter().any(|item| item == tag) {
            return false;
        }
    }
    if mono {
        return frame.sat < 0.15;
    }
    if let Some(hue) = hue {
        return frame.hue == Some(hue);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_archive_has_frames() {
        let gallery = data();
        assert_eq!(gallery.total, 258);
        assert_eq!(gallery.sheet_count, 8);
        assert_eq!(gallery.sheets[7].total, 6);
        assert_eq!(gallery.sheets[0].frames.len(), 36);
        assert_eq!(gallery.sheets[0].frames[0].number, "001");
        assert_eq!(gallery.sheets[7].frames[0].number, "253");
    }

    #[test]
    fn curation_tags_beat_feed_tags() {
        let photos = r#"{"photos":[
            {"id":"1","feed_tags":["wrong"]},
            {"id":"2","feed_tags":["kept"]}
        ]}"#;
        let curation = r#"{"photos":{"1":{"tags":["nature","sky"]}}}"#;
        let gallery = build(photos, curation);
        assert_eq!(
            gallery
                .tags
                .iter()
                .find(|tag| tag.name == "nature")
                .unwrap()
                .count,
            1
        );
        assert_eq!(
            gallery
                .tags
                .iter()
                .find(|tag| tag.name == "kept")
                .unwrap()
                .count,
            1
        );
        assert!(gallery.tags.iter().all(|tag| tag.name != "wrong"));
    }

    #[test]
    fn camera_names_are_not_labels() {
        assert!(is_camera_name("IMG_1234"));
        assert!(is_camera_name("DSC0001"));
        assert!(is_camera_name("123456"));
        assert!(is_camera_name("photo"));
        assert!(!is_camera_name("the sea"));
    }

    #[test]
    fn filters_match_the_dataset() {
        let gallery = data();
        let all = filter_counts(&gallery, None, false, None);
        assert_eq!(all, gallery.total);
        let mono = filter_counts(&gallery, None, true, None);
        assert_eq!(mono, gallery.mono_count);
        if let Some(first) = gallery.swatches.first() {
            let count = filter_counts(&gallery, Some(first.hue), false, None);
            assert_eq!(count, first.count);
        }
        let untagged = filter_counts(&gallery, None, false, Some("__untagged"));
        assert_eq!(untagged, gallery.untagged);
    }

    #[test]
    fn palette_scoring_picks_the_most_saturated_colour() {
        let rgb = most_saturated(&["#101010".to_string(), "#ff2000".to_string()]).unwrap();
        assert_eq!(rgb, [255, 32, 0]);
        assert_eq!(hue_name(25.0), "rust");
    }

    #[test]
    fn generated_frames_have_every_template_field() {
        let gallery = data();
        let frame = &gallery.sheets[0].frames[0];
        assert!(!frame.id.is_empty());
        assert!(!frame.alt.is_empty());
        assert!(!frame.label.is_empty());
        assert!(!frame.meta.is_empty());
        assert!(frame.src.starts_with("https://"));
    }
}
