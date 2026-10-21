use serde::Deserialize;
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

/// Generated from Flickr by `tools/gallery/sync.py`.
const PHOTOS: &str = include_str!("../gallery/photos.json");
/// Hand-written, survives every sync.
const CURATION: &str = include_str!("../gallery/curation.json");

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
    /// Camera, lens, exposure — only present when a Flickr API key is
    /// available to the sync tool.
    #[serde(default)]
    exif: Option<Map<String, Value>>,
}

/// The gallery dataset the template embeds, with curation applied.
///
/// Read from disk so curation edits show up on the next request, and so a
/// deploy can refresh `photos.json` without rebuilding the binary. The merged
/// result is cached until either file's mtime changes.
pub fn photos_json() -> String {
    type Cache = Option<(SystemTime, SystemTime, String)>;
    static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(None));

    let photos_mtime = mtime("gallery/photos.json");
    let curation_mtime = mtime("gallery/curation.json");

    let mut guard = cache.lock().unwrap_or_else(|error| error.into_inner());
    if let Some((cached_photos, cached_curation, json)) = guard.as_ref() {
        if *cached_photos == photos_mtime && *cached_curation == curation_mtime {
            return json.clone();
        }
    }

    let photos = fs::read_to_string("gallery/photos.json").unwrap_or_else(|_| PHOTOS.to_string());
    let curation =
        fs::read_to_string("gallery/curation.json").unwrap_or_else(|_| CURATION.to_string());
    let json = merge(&photos, &curation);
    *guard = Some((photos_mtime, curation_mtime, json.clone()));
    json
}

fn mtime(path: &str) -> SystemTime {
    fs::metadata(Path::new(path))
        .and_then(|meta| meta.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH)
}

/// Number of frames in the archive.
pub fn count() -> usize {
    serde_json::from_str::<Value>(&photos_json())
        .ok()
        .and_then(|data| data.get("photos").and_then(Value::as_array).map(Vec::len))
        .unwrap_or(0)
}

fn merge(photos: &str, curation: &str) -> String {
    let mut data: Value = serde_json::from_str(photos).unwrap_or_else(|_| json!({ "photos": [] }));
    let curated: Curation = serde_json::from_str(curation).unwrap_or_default();

    if let Some(items) = data.get_mut("photos").and_then(Value::as_array_mut) {
        for item in items {
            let Some(id) = item.get("id").and_then(Value::as_str).map(str::to_owned) else {
                continue;
            };

            if let Some(c) = curated.photos.get(&id) {
                if !c.tags.is_empty() {
                    item["tags"] = json!(c.tags);
                }
                if let Some(kind) = &c.r#type {
                    item["type"] = json!(kind);
                }
                if let Some(caption) = &c.caption {
                    item["caption"] = json!(caption);
                }
                if let Some(alt) = &c.alt {
                    item["alt"] = json!(alt);
                }
                if let Some(exif) = &c.exif {
                    item["exif"] = Value::Object(exif.clone());
                }
            }

            if item.get("type").is_none() {
                item["type"] = json!("photography");
            }
            if item.get("tags").is_none() {
                let fallback = item.get("feed_tags").cloned().unwrap_or_else(|| json!([]));
                item["tags"] = fallback;
            }
        }
    }

    // A photo title must never be able to close the <script> element the
    // dataset is embedded in.
    serde_json::to_string(&data)
        .unwrap_or_else(|_| "{\"photos\":[]}".to_string())
        .replace('<', "\\u003c")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_archive_has_frames() {
        assert!(count() > 100, "gallery archive looks empty");
    }

    #[test]
    fn curation_tags_beat_feed_tags() {
        let photos =
            r#"{"photos":[{"id":"1","feed_tags":["wrong"]},{"id":"2","feed_tags":["kept"]}]}"#;
        let curation = r#"{"photos":{"1":{"tags":["nature","sky"]}}}"#;
        let merged: Value = serde_json::from_str(&merge(photos, curation)).unwrap();
        assert_eq!(merged["photos"][0]["tags"], json!(["nature", "sky"]));
        assert_eq!(merged["photos"][1]["tags"], json!(["kept"]));
        assert_eq!(merged["photos"][0]["type"], json!("photography"));
    }

    #[test]
    fn a_hostile_title_cannot_close_the_script_tag() {
        let merged = merge(
            r#"{"photos":[{"id":"1","title":"</script><script>alert(1)</script>"}]}"#,
            "{}",
        );
        assert!(!merged.contains("</script>"));
        assert!(merged.contains("\\u003c/script>"));
    }
}
