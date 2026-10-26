use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

const FALLBACK_MANIFEST: &str = include_str!("../music/tracks.json");
const DIR: &str = "static/audio/music";
const MANIFEST: &str = "music/tracks.json";
const EXTENSIONS: &[&str] = &["mp3", "m4a", "aac", "ogg", "oga", "opus", "flac", "wav"];

#[derive(Deserialize, Default)]
struct Manifest {
    #[serde(default)]
    order: Vec<String>,
    #[serde(default)]
    tracks: BTreeMap<String, Entry>,
}

#[derive(Deserialize, Default)]
struct Entry {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    artist: Option<String>,
    #[serde(default)]
    album: Option<String>,
    #[serde(default)]
    duration: Option<f64>,
    #[serde(default)]
    soundcloud: Option<String>,
    #[serde(default)]
    hidden: bool,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Track {
    pub file: String,
    pub src: String,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub duration: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub soundcloud: Option<String>,
    pub bytes: u64,
}

pub fn tracks() -> (String, usize) {
    let manifest = fs::read_to_string(MANIFEST).unwrap_or_else(|_| FALLBACK_MANIFEST.to_string());
    let list = arrange(&manifest, scan());
    let count = list.len();
    let json = json!({ "tracks": list }).to_string().replace("</", "<\\/");
    (json, count)
}

fn version(path: &Path) -> u64 {
    fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|since| since.as_secs())
        .unwrap_or(0)
}

fn scan() -> Vec<Track> {
    let mut found = Vec::new();
    let Ok(entries) = fs::read_dir(DIR) else {
        return found;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(file) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if file.starts_with('.') {
            continue;
        }
        let Some(extension) = path.extension().and_then(|ext| ext.to_str()) else {
            continue;
        };
        if !is_audio(extension) {
            continue;
        }

        let stem = file.rsplit_once('.').map(|(stem, _)| stem).unwrap_or(file);
        let (artist, title) = parse_name(stem);
        found.push(Track {
            file: file.to_string(),
            src: format!(
                "/static/audio/music/{}?v={}",
                encode_segment(file),
                version(&path)
            ),
            title,
            artist,
            album: None,
            duration: None,
            soundcloud: None,
            bytes: fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0),
        });
    }

    found
}

fn arrange(manifest_text: &str, mut found: Vec<Track>) -> Vec<Track> {
    let manifest: Manifest = serde_json::from_str(manifest_text).unwrap_or_default();

    found.retain(|track| {
        !manifest
            .tracks
            .get(&track.file)
            .map(|entry| entry.hidden)
            .unwrap_or(false)
    });

    for track in found.iter_mut() {
        let Some(entry) = manifest.tracks.get(&track.file) else {
            continue;
        };
        if let Some(value) = &entry.title {
            track.title = value.clone();
        }
        if let Some(value) = &entry.artist {
            track.artist = value.clone();
        }
        track.album = entry.album.clone();
        track.duration = entry.duration;
        track.soundcloud = entry.soundcloud.clone();
    }

    found.sort_by_key(|track| track.file.to_lowercase());

    let mut ordered = Vec::with_capacity(found.len());
    for name in &manifest.order {
        if let Some(index) = found.iter().position(|track| &track.file == name) {
            ordered.push(found.remove(index));
        }
    }
    ordered.extend(found);
    ordered
}

fn is_audio(extension: &str) -> bool {
    EXTENSIONS
        .iter()
        .any(|known| extension.eq_ignore_ascii_case(known))
}

fn parse_name(stem: &str) -> (String, String) {
    let name = strip_track_number(stem.trim()).trim();
    for separator in [" - ", " \u{2013} ", " \u{2014} "] {
        if let Some((artist, title)) = name.split_once(separator) {
            let artist = artist.trim();
            let title = title.trim();
            if !artist.is_empty() && !title.is_empty() {
                return (artist.to_string(), title.to_string());
            }
        }
    }
    (String::new(), name.to_string())
}

fn strip_track_number(name: &str) -> &str {
    let digits = name.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits == 0 || digits > 3 {
        return name;
    }
    let rest = name[digits..].trim_start();
    let Some(marker) = rest.chars().next() else {
        return name;
    };
    if marker != '.' && marker != '-' && marker != '_' {
        return name;
    }
    let after = &rest[marker.len_utf8()..];
    let after = after.trim_start_matches(|c: char| c == ' ' || c == '.' || c == '-' || c == '_');
    if after.is_empty() {
        name
    } else {
        after
    }
}

fn encode_segment(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for &byte in name.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(file: &str, artist: &str, title: &str) -> Track {
        Track {
            file: file.to_string(),
            src: format!("/static/audio/music/{file}"),
            title: title.to_string(),
            artist: artist.to_string(),
            album: None,
            duration: None,
            soundcloud: None,
            bytes: 0,
        }
    }

    #[test]
    fn filenames_split_into_artist_and_title() {
        assert_eq!(
            parse_name("Vitalic - Polkamatic"),
            ("Vitalic".to_string(), "Polkamatic".to_string())
        );
        assert_eq!(
            parse_name("01 - Vitalic - Polkamatic"),
            ("Vitalic".to_string(), "Polkamatic".to_string())
        );
        assert_eq!(
            parse_name("07. Miss Kittin - The Hacker"),
            ("Miss Kittin".to_string(), "The Hacker".to_string())
        );
        assert_eq!(
            parse_name("A \u{2013} B"),
            ("A".to_string(), "B".to_string())
        );
    }

    #[test]
    fn titles_that_start_with_numbers_are_left_alone() {
        assert_eq!(
            parse_name("99 Luftballons"),
            (String::new(), "99 Luftballons".to_string())
        );
        assert_eq!(parse_name("1999"), (String::new(), "1999".to_string()));
        assert_eq!(
            parse_name("untitled"),
            (String::new(), "untitled".to_string())
        );
    }

    #[test]
    fn only_audio_extensions_count() {
        assert!(is_audio("MP3"));
        assert!(is_audio("flac"));
        assert!(is_audio("m4a"));
        assert!(!is_audio("jpg"));
        assert!(!is_audio("txt"));
    }

    #[test]
    fn file_names_are_url_encoded() {
        assert_eq!(
            encode_segment("Artist - T\u{ef}tle.mp3"),
            "Artist%20-%20T%C3%AFtle.mp3"
        );
    }

    #[test]
    fn manifest_orders_overrides_and_hides_tracks() {
        let manifest = r#"{
            "order": ["b.mp3"],
            "tracks": {
                "a.mp3": { "artist": "Override", "title": "First", "duration": 12.5 },
                "c.mp3": { "hidden": true }
            }
        }"#;
        let found = vec![
            track("a.mp3", "", "a"),
            track("b.mp3", "B", "b"),
            track("c.mp3", "C", "c"),
        ];
        let arranged = arrange(manifest, found);

        assert_eq!(arranged.len(), 2);
        assert_eq!(arranged[0].file, "b.mp3");
        assert_eq!(arranged[1].file, "a.mp3");
        assert_eq!(arranged[1].artist, "Override");
        assert_eq!(arranged[1].title, "First");
        assert_eq!(arranged[1].duration, Some(12.5));
    }

    #[test]
    fn manifest_can_attach_a_soundcloud_link() {
        let manifest = r#"{
            "tracks": {
                "a.mp3": { "soundcloud": "https://soundcloud.com/artist/track" }
            }
        }"#;
        let arranged = arrange(manifest, vec![track("a.mp3", "A", "a")]);

        assert_eq!(
            arranged[0].soundcloud,
            Some("https://soundcloud.com/artist/track".to_string())
        );
    }

    #[test]
    fn a_broken_manifest_still_lists_tracks() {
        let arranged = arrange("{", vec![track("a.mp3", "A", "a")]);
        assert_eq!(arranged.len(), 1);
        assert_eq!(arranged[0].title, "a");
    }
}
