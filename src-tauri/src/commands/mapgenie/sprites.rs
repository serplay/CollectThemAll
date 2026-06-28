//! Cutting the big "sprite sheet" image up into one small PNG per marker icon.
//!
//! Cybersecurity note: we are processing an image file that came from the
//! internet, so it counts as untrusted input. Before cropping we bounds-check
//! every rectangle against the real image size — if a crop would go outside the
//! picture we just skip it instead of crashing. Validating sizes/offsets before
//! using them is exactly how we avoid the out-of-bounds bugs we read about.

use std::collections::HashMap;
use std::path::PathBuf;

use super::models::MarkerSpriteEntry;

/// Crops each named region out of the sprite sheet and saves it as its own PNG.
pub fn slice_marker_sprites(
    sprite_path: &PathBuf,
    out_dir: &PathBuf,
    markers: &HashMap<String, MarkerSpriteEntry>,
) -> Result<(), String> {
    let sheet = image::open(sprite_path).map_err(|e| e.to_string())?;
    let (sheet_w, sheet_h) = (sheet.width(), sheet.height());

    for (name, entry) in markers {
        // Skip empty rectangles — nothing to crop.
        if entry.width == 0 || entry.height == 0 {
            continue;
        }
        // Defensive bounds check: never read pixels outside the actual image.
        if entry.x + entry.width > sheet_w || entry.y + entry.height > sheet_h {
            continue;
        }

        let cropped = sheet.crop_imm(entry.x, entry.y, entry.width, entry.height);
        let out_path = out_dir.join(format!("{}.png", name));
        cropped.save(&out_path).map_err(|e| e.to_string())?;
    }

    Ok(())
}
