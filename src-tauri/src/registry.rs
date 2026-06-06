// Scans public/sprites/*_sheet.png and builds the AnimationEntry list.
// Metadata (fps, loop_mode) is hardcoded per known id; frame_count/width/height
// come from the PNG IHDR chunk (no extra crate dep).

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use crate::types::{AnimationEntry, AppError, LoopMode};

#[derive(Copy, Clone)]
struct AnimationMeta {
    fps: u32,
    loop_mode: LoopMode,
}

fn known_meta() -> HashMap<&'static str, AnimationMeta> {
    let mut m = HashMap::new();
    m.insert("touch_nose", AnimationMeta { fps: 25, loop_mode: LoopMode::Infinite });
    m.insert("think", AnimationMeta { fps: 25, loop_mode: LoopMode::Infinite });
    m.insert("poop", AnimationMeta { fps: 25, loop_mode: LoopMode::Once });
    m
}

pub fn list_animations(sprites_dir: &Path) -> Result<Vec<AnimationEntry>, AppError> {
    if !sprites_dir.exists() {
        return Err(AppError::frames_missing(format!(
            "sprites dir not found: {}",
            sprites_dir.display()
        )));
    }

    let entries_iter = match fs::read_dir(sprites_dir) {
        Ok(r) => r,
        Err(e) => return Err(AppError::internal(format!("read_dir failed: {e}"))),
    };

    let meta = known_meta();
    let mut entries: Vec<AnimationEntry> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    for dir_entry in entries_iter.flatten() {
        let path = dir_entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else { continue };
        if !name.ends_with("_sheet.png") {
            continue;
        }
        let id = name.trim_end_matches("_sheet.png").to_string();

        let (sheet_w, sheet_h) = match read_png_dimensions(&path) {
            Ok(d) => d,
            Err(_) => {
                // AC-F2.3: deleted/malformed sheet → no [PetError] log, just skip.
                continue;
            }
        };
        if sheet_w == 0 || sheet_h == 0 {
            continue;
        }
        if !seen.insert(id.clone()) {
            return Err(AppError::internal(format!("duplicate animation id: {id}")));
        }

        let frame_width = sheet_h;
        let frame_height = sheet_h;
        let frame_count = sheet_w / sheet_h;
        let m = meta
            .get(id.as_str())
            .copied()
            .unwrap_or(AnimationMeta { fps: 25, loop_mode: LoopMode::Infinite });

        entries.push(AnimationEntry {
            id: id.clone(),
            sheet_path: format!("/sprites/{id}_sheet.png"),
            frame_count,
            frame_width,
            frame_height,
            fps: m.fps,
            loop_mode: m.loop_mode,
        });
    }

    entries.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(entries)
}

fn read_png_dimensions(path: &Path) -> io::Result<(u32, u32)> {
    let mut f = fs::File::open(path)?;
    let mut header = [0u8; 24];
    f.read_exact(&mut header)?;
    if &header[0..8] != b"\x89PNG\r\n\x1a\n" {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "not a PNG"));
    }
    if &header[12..16] != b"IHDR" {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "missing IHDR"));
    }
    let width = u32::from_be_bytes([header[16], header[17], header[18], header[19]]);
    let height = u32::from_be_bytes([header[20], header[21], header[22], header[23]]);
    Ok((width, height))
}

/// Check whether an animation id exists in the registry (R10: validate LLM output).
pub fn is_known_animation(id: &str) -> bool {
    let sprites_dir = locate_sprites_dir();
    match list_animations(&sprites_dir) {
        Ok(entries) => entries.iter().any(|e| e.id == id),
        Err(_) => false,
    }
}

pub fn locate_sprites_dir() -> PathBuf {
    // Try common dev locations first.
    let candidates = [
        PathBuf::from("public/sprites"),
        PathBuf::from("../public/sprites"),
        PathBuf::from("../../public/sprites"),
    ];
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    // Last resort: walk up from cwd.
    if let Ok(cwd) = std::env::current_dir() {
        for ancestor in cwd.ancestors() {
            let candidate = ancestor.join("public").join("sprites");
            if candidate.exists() {
                return candidate;
            }
        }
    }
    PathBuf::from("public/sprites")
}
