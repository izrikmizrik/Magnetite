//! The pictures on the touch buttons (game/interface/touch.rs): a built-in set, and the
//! player's own, from a `touch-icons` folder in the game folder (`HORIZON_TOUCH_ICONS` names
//! another): `<action>.png` (or `.jpg`, `.webp`), the action being the button's id: `jump`,
//! `sneak`, `use`, `hit`, `inventory`...
//!
//! Ported from the iOS fork of Horizon (MIT licence), where it is `touch_icons.rs`; the
//! built-in pictures were drawn for this project (`tools/ios/gen_touch_icons.py` there).
//!
//! The player's picture for an action is the whole button (`touch/user/<action>`, its corners
//! rounded like the button's), drawn with no plate under it. The plate (`plate`) and the stick's
//! pictures (`stick_base`, `stick_knob`) are replaced in place (`touch/<name>`).
//!
//! Every picture is made 256 x 256 for the atlas (`touch/<action>`): one bigger than that is
//! shrunk with bilinear filtering, one smaller is enlarged by nearest neighbour (hard pixels stay
//! hard), and one that is not square keeps its proportions, centred on a clear square. A resource
//! pack's `textures/gui/sprites/touch/<action>.png` also works; the player's folder wins over the
//! pack, the pack over the built-in one.
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// What a picture is made: the side of its square.
pub const SIDE: u32 = 256;
/// The most pictures taken from the player's folder.
const MAX_USER_ICONS: usize = 48;
/// A bigger file isn't read (a photo of 12 megapixels is about 5 MB as a JPEG).
const MAX_FILE_BYTES: u64 = 32 * 1024 * 1024;

type Image = (String, u32, u32, Vec<u8>);

/// The built-in pictures, by action id.
const BUILT_IN: &[(&str, &[u8])] = &[
    ("jump", include_bytes!("../assets/touch/jump.png")),
    ("sneak", include_bytes!("../assets/touch/sneak.png")),
    ("use", include_bytes!("../assets/touch/use.png")),
    ("hit", include_bytes!("../assets/touch/hit.png")),
    ("inventory", include_bytes!("../assets/touch/inventory.png")),
    ("chat", include_bytes!("../assets/touch/chat.png")),
    ("drop", include_bytes!("../assets/touch/drop.png")),
    ("camera", include_bytes!("../assets/touch/camera.png")),
    (
        "hotbar_previous",
        include_bytes!("../assets/touch/hotbar_previous.png"),
    ),
    (
        "hotbar_next",
        include_bytes!("../assets/touch/hotbar_next.png"),
    ),
    ("sprint", include_bytes!("../assets/touch/sprint.png")),
    (
        "swap_hands",
        include_bytes!("../assets/touch/swap_hands.png"),
    ),
    (
        "pick_block",
        include_bytes!("../assets/touch/pick_block.png"),
    ),
    (
        "advancements",
        include_bytes!("../assets/touch/advancements.png"),
    ),
    (
        "player_list",
        include_bytes!("../assets/touch/player_list.png"),
    ),
    ("command", include_bytes!("../assets/touch/command.png")),
    // The button's rounded plate and the move stick's base ring and ball (game/interface/touch.rs).
    ("plate", include_bytes!("../assets/touch/plate.png")),
    (
        "stick_base",
        include_bytes!("../assets/touch/stick_base.png"),
    ),
    (
        "stick_knob",
        include_bytes!("../assets/touch/stick_knob.png"),
    ),
];

/// The atlas sprite of an action's picture.
pub fn sprite(action: &str) -> String {
    format!("touch/{action}")
}

/// What the player's folder may replace in place: the plate and the stick's two pictures.
const SHARED: [&str; 3] = ["plate", "stick_base", "stick_knob"];

/// The sprite of the player's own file `<id>`: a button's picture is its own sprite
/// (`touch/user/<id>`), the shared ones keep their names.
fn user_sprite(id: &str) -> String {
    if SHARED.contains(&id) {
        sprite(id)
    } else {
        format!("touch/user/{id}")
    }
}

/// The 256 x 256 `rgba`'s corners rounded (radius 40, soft by a pixel), like the button's.
fn round_corners(rgba: &mut [u8]) {
    const R: f32 = 40.0;
    let side = SIDE as usize;
    for (cx, cy) in [
        (R, R),
        (side as f32 - R, R),
        (R, side as f32 - R),
        (side as f32 - R, side as f32 - R),
    ] {
        let (x0, y0) = (
            if cx < 128.0 { 0 } else { side - R as usize },
            if cy < 128.0 { 0 } else { side - R as usize },
        );
        for y in y0..y0 + R as usize {
            for x in x0..x0 + R as usize {
                let d = ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2)).sqrt();
                let keep = (R - d + 0.5).clamp(0.0, 1.0);
                let a = &mut rgba[(y * side + x) * 4 + 3];
                *a = (f32::from(*a) * keep).round() as u8;
            }
        }
    }
}

/// The folder the player's pictures are read from.
pub fn user_dir() -> PathBuf {
    match std::env::var_os("HORIZON_TOUCH_ICONS") {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => PathBuf::from("touch-icons"),
    }
}

/// The 256 x 256 RGBA pixels of a `w` x `h` RGBA image, as the buttons use it.
pub fn normalize(w: u32, h: u32, rgba: &[u8]) -> Vec<u8> {
    let side = SIDE as usize;
    let mut out = vec![0u8; side * side * 4];
    let (w, h) = (w as usize, h as usize);
    if w == 0 || h == 0 || rgba.len() < w * h * 4 {
        return out;
    }
    // The longer side becomes 256, the other keeps the proportions.
    let scale = side as f32 / w.max(h) as f32;
    let nw = ((w as f32 * scale).round() as usize).clamp(1, side);
    let nh = ((h as f32 * scale).round() as usize).clamp(1, side);
    let (ox, oy) = ((side - nw) / 2, (side - nh) / 2);
    for y in 0..nh {
        for x in 0..nw {
            let px = if scale < 1.0 {
                bilinear(
                    rgba,
                    w,
                    h,
                    (x as f32 + 0.5) / scale - 0.5,
                    (y as f32 + 0.5) / scale - 0.5,
                )
            } else {
                // Nearest neighbour: the source pixel under the middle of this one.
                let sx = (((x as f32 + 0.5) / scale) as usize).min(w - 1);
                let sy = (((y as f32 + 0.5) / scale) as usize).min(h - 1);
                let at = (sy * w + sx) * 4;
                [rgba[at], rgba[at + 1], rgba[at + 2], rgba[at + 3]]
            };
            let at = ((oy + y) * side + ox + x) * 4;
            out[at..at + 4].copy_from_slice(&px);
        }
    }
    out
}

/// A bilinear sample at (`fx`, `fy`) in source pixels, with alpha taken into account (a clear
/// pixel's colour doesn't bleed into the edge).
fn bilinear(rgba: &[u8], w: usize, h: usize, fx: f32, fy: f32) -> [u8; 4] {
    let fx = fx.clamp(0.0, (w - 1) as f32);
    let fy = fy.clamp(0.0, (h - 1) as f32);
    let (x0, y0) = (fx.floor() as usize, fy.floor() as usize);
    let (x1, y1) = ((x0 + 1).min(w - 1), (y0 + 1).min(h - 1));
    let (tx, ty) = (fx - x0 as f32, fy - y0 as f32);
    let mut acc = [0f32; 4];
    for (sx, sy, weight) in [
        (x0, y0, (1.0 - tx) * (1.0 - ty)),
        (x1, y0, tx * (1.0 - ty)),
        (x0, y1, (1.0 - tx) * ty),
        (x1, y1, tx * ty),
    ] {
        let at = (sy * w + sx) * 4;
        let a = f32::from(rgba[at + 3]) / 255.0;
        for c in 0..3 {
            acc[c] += f32::from(rgba[at + c]) * a * weight;
        }
        acc[3] += a * weight;
    }
    if acc[3] <= 0.0 {
        return [0, 0, 0, 0];
    }
    let to_u8 = |v: f32| v.round().clamp(0.0, 255.0) as u8;
    [
        to_u8(acc[0] / acc[3]),
        to_u8(acc[1] / acc[3]),
        to_u8(acc[2] / acc[3]),
        to_u8(acc[3] * 255.0),
    ]
}

/// Whether `name` can be an action's id (what a file may be called).
fn is_action_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 32
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

/// The pictures in the player's folder, made 256 x 256, by sprite name. A file that can't be read
/// is skipped (and logged).
pub fn user_icons(dir: &Path) -> Vec<Image> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    files.sort();
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for path in files {
        if out.len() >= MAX_USER_ICONS {
            log::warn!(
                "touch icons: more than {MAX_USER_ICONS} in {}: the rest are left out",
                dir.display()
            );
            break;
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase)
            .unwrap_or_default();
        if !matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "webp") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let stem = stem.to_ascii_lowercase();
        if !is_action_name(&stem) || !seen.insert(stem.clone()) {
            continue;
        }
        if std::fs::metadata(&path).is_ok_and(|m| m.len() > MAX_FILE_BYTES) {
            log::warn!("touch icons: {} is too big", path.display());
            continue;
        }
        let decoded = std::fs::read(&path)
            .map_err(|e| e.to_string())
            .and_then(|bytes| crate::hud::decode_image_bytes(&bytes).map_err(|e| e.to_string()));
        match decoded {
            Ok((w, h, rgba)) => {
                let mut pixels = normalize(w, h, &rgba);
                if !SHARED.contains(&stem.as_str()) {
                    round_corners(&mut pixels);
                }
                out.push((user_sprite(&stem), SIDE, SIDE, pixels));
            }
            Err(e) => log::warn!("touch icons: {} skipped: {e}", path.display()),
        }
    }
    out
}

/// Whether `id` can name an action's picture file.
pub fn valid_id(id: &str) -> bool {
    is_action_name(id)
}

/// Makes `bytes` (a PNG, JPEG or WebP, a photo from the player's library) the picture of action
/// `id`: made 256 x 256 as the game reads them and written as `<dir>/<id>.png`, replacing the
/// one there. The game reads the folder at its start, so it shows after a restart.
pub fn save_user_icon(dir: &Path, id: &str, bytes: &[u8]) -> Result<(), String> {
    if !is_action_name(id) {
        return Err(format!("{id} is not a picture's name"));
    }
    let (w, h, rgba) = crate::hud::decode_image_bytes(bytes).map_err(|e| e.to_string())?;
    let pixels = normalize(w, h, &rgba);
    let mut png_bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png_bytes, SIDE, SIDE);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
        writer
            .write_image_data(&pixels)
            .map_err(|e| e.to_string())?;
    }
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let target = dir.join(format!("{id}.png"));
    // Written beside it and renamed over it: a stop in the middle leaves the old picture.
    let partial = dir.join(format!(".{id}.png.part"));
    std::fs::write(&partial, &png_bytes).map_err(|e| format!("{}: {e}", partial.display()))?;
    std::fs::rename(&partial, &target).map_err(|e| format!("{}: {e}", target.display()))?;
    // A JPEG or WebP of the same name would win over the new PNG in the folder's order.
    for ext in ["jpg", "jpeg", "webp"] {
        let _ = std::fs::remove_file(dir.join(format!("{id}.{ext}")));
    }
    Ok(())
}

/// Takes the player's own picture of action `id` away (`<dir>/<id>.png`, `.jpg`, `.jpeg` or
/// `.webp`): the button has its built-in picture again at the game's next start. Whether there
/// was one.
pub fn remove_user_icon(dir: &Path, id: &str) -> bool {
    if !is_action_name(id) {
        return false;
    }
    let mut removed = false;
    for ext in ["png", "jpg", "jpeg", "webp"] {
        removed |= std::fs::remove_file(dir.join(format!("{id}.{ext}"))).is_ok();
    }
    removed
}

/// Adds the buttons' pictures to the atlas's images: the player's, which replace a pack's of the
/// same name, then the built-in ones for the actions that have none yet.
pub fn add(images: &mut Vec<Image>) {
    let user = user_icons(&user_dir());
    let names: HashSet<String> = user.iter().map(|i| i.0.clone()).collect();
    images.retain(|i| !names.contains(&i.0));
    images.extend(user);
    for (action, bytes) in BUILT_IN {
        let name = sprite(action);
        if images.iter().any(|i| i.0 == name) {
            continue;
        }
        match crate::hud::decode_bytes(bytes) {
            Ok((w, h, rgba)) => images.push((name, SIDE, SIDE, normalize(w, h, &rgba))),
            Err(e) => log::warn!("touch icons: built-in {action}: {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(w: usize, h: usize, px: [u8; 4]) -> Vec<u8> {
        px.iter().copied().cycle().take(w * h * 4).collect()
    }
    fn at(img: &[u8], x: usize, y: usize) -> [u8; 4] {
        let i = (y * SIDE as usize + x) * 4;
        [img[i], img[i + 1], img[i + 2], img[i + 3]]
    }

    #[test]
    fn a_big_picture_is_shrunk_to_the_square() {
        let out = normalize(1024, 1024, &solid(1024, 1024, [200, 100, 50, 255]));
        assert_eq!(out.len(), 256 * 256 * 4);
        assert_eq!(at(&out, 0, 0), [200, 100, 50, 255]);
        assert_eq!(at(&out, 255, 255), [200, 100, 50, 255]);
    }

    #[test]
    fn bilinear_shrinking_blends_neighbours() {
        // Black and white columns 2 pixels wide, shrunk by 2: each result pixel is between.
        let (w, h) = (512usize, 512usize);
        let mut px = vec![0u8; w * h * 4];
        for y in 0..h {
            for x in 0..w {
                let v = if x % 2 == 0 { 0 } else { 255 };
                px[(y * w + x) * 4..][..4].copy_from_slice(&[v, v, v, 255]);
            }
        }
        let out = normalize(w as u32, h as u32, &px);
        let mid = at(&out, 100, 100)[0];
        assert!((100..=155).contains(&mid), "{mid}");
    }

    #[test]
    fn a_small_picture_is_enlarged_with_hard_pixels() {
        // 2 x 2 (red, green / blue, white) to 256: four solid quarters, nothing blended.
        let px = [
            255, 0, 0, 255, 0, 255, 0, 255, //
            0, 0, 255, 255, 255, 255, 255, 255,
        ];
        let out = normalize(2, 2, &px);
        assert_eq!(at(&out, 10, 10), [255, 0, 0, 255]);
        assert_eq!(at(&out, 245, 10), [0, 255, 0, 255]);
        assert_eq!(at(&out, 10, 245), [0, 0, 255, 255]);
        assert_eq!(at(&out, 245, 245), [255, 255, 255, 255]);
        assert_eq!(at(&out, 127, 10), [255, 0, 0, 255]);
        assert_eq!(at(&out, 128, 10), [0, 255, 0, 255]);
    }

    #[test]
    fn a_wide_picture_keeps_its_proportions() {
        // 512 x 128 becomes 256 x 64, centred: the rows above and below are clear.
        let out = normalize(512, 128, &solid(512, 128, [9, 9, 9, 255]));
        assert_eq!(at(&out, 128, 0)[3], 0);
        assert_eq!(at(&out, 128, 127), [9, 9, 9, 255]);
        assert_eq!(at(&out, 128, 255)[3], 0);
        assert_eq!(at(&out, 0, 127), [9, 9, 9, 255]);
    }

    #[test]
    fn clear_pixels_do_not_bleed_colour_into_an_edge() {
        // Left half opaque white, right half clear red: shrunk, the edge stays white.
        let (w, h) = (512usize, 512usize);
        let mut px = vec![0u8; w * h * 4];
        for y in 0..h {
            for x in 0..w {
                let v: [u8; 4] = if x < w / 2 {
                    [255, 255, 255, 255]
                } else {
                    [255, 0, 0, 0]
                };
                px[(y * w + x) * 4..][..4].copy_from_slice(&v);
            }
        }
        let out = normalize(w as u32, h as u32, &px);
        let edge = at(&out, 127, 50);
        assert!(edge[3] > 0 && edge[1] > 250 && edge[2] > 250, "{edge:?}");
    }

    #[test]
    fn the_built_in_pictures_are_all_there_and_square() {
        let mut images = Vec::new();
        add(&mut images);
        assert!(images.iter().any(|i| i.0 == "touch/jump"));
        assert_eq!(
            images.iter().filter(|i| i.0.starts_with("touch/")).count(),
            BUILT_IN.len()
        );
        assert!(
            images
                .iter()
                .all(|i| (i.1, i.2) == (SIDE, SIDE) && i.3.len() == 256 * 256 * 4)
        );
    }

    #[test]
    fn a_picked_photo_becomes_the_buttons_picture() {
        let dir = std::env::temp_dir().join(format!("touch-save-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        // A 4 x 2 PNG, left half red, right half blue.
        let mut png_bytes = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut png_bytes, 4, 2);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            let mut w = enc.write_header().unwrap();
            let row = [
                255, 0, 0, 255, 255, 0, 0, 255, 0, 0, 255, 255, 0, 0, 255, 255,
            ];
            w.write_image_data(&[row, row].concat()).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("f3.jpg"), b"old").unwrap();
        save_user_icon(&dir, "f3", &png_bytes).unwrap();
        // Read back like the game reads the folder: one picture, 256 x 256, the old jpg gone.
        let icons = user_icons(&dir);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(icons.len(), 1);
        assert_eq!(icons[0].0, "touch/user/f3");
        assert_eq!((icons[0].1, icons[0].2), (SIDE, SIDE));
        assert_eq!(at(&icons[0].3, 10, 128), [255, 0, 0, 255]);
        assert_eq!(at(&icons[0].3, 245, 128), [0, 0, 255, 255]);
        assert!(save_user_icon(&dir, "../evil", &png_bytes).is_err());
        assert!(save_user_icon(&dir, "ok", b"not an image").is_err());
    }

    #[test]
    fn a_players_picture_can_be_taken_away() {
        let dir = std::env::temp_dir().join(format!("touch-remove-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("f3.png"), b"x").unwrap();
        std::fs::write(dir.join("f3.jpg"), b"x").unwrap();
        std::fs::write(dir.join("jump.png"), b"x").unwrap();
        assert!(remove_user_icon(&dir, "f3"));
        // Both files of that action are gone, the other action's stays; nothing left to remove.
        assert!(!dir.join("f3.png").exists() && !dir.join("f3.jpg").exists());
        assert!(dir.join("jump.png").exists());
        assert!(!remove_user_icon(&dir, "f3"));
        assert!(!remove_user_icon(&dir, "../jump"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_players_picture_is_its_own_sprite_with_rounded_corners() {
        let dir = std::env::temp_dir().join(format!("touch-own-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut png = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut png, 1, 1);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            enc.write_header()
                .unwrap()
                .write_image_data(&[200, 10, 10, 255])
                .unwrap();
        }
        // A button's picture and a shared one.
        std::fs::write(dir.join("hit.png"), &png).unwrap();
        std::fs::write(dir.join("plate.png"), &png).unwrap();
        let icons = user_icons(&dir);
        let _ = std::fs::remove_dir_all(&dir);
        let names: Vec<&str> = icons.iter().map(|i| i.0.as_str()).collect();
        assert_eq!(names, ["touch/user/hit", "touch/plate"]);
        let hit = &icons[0].3;
        // The corner is clear, the middle and the edges' middles are not.
        assert_eq!(at(hit, 0, 0)[3], 0);
        assert_eq!(at(hit, 255, 255)[3], 0);
        assert_eq!(at(hit, 128, 128), [200, 10, 10, 255]);
        assert_eq!(at(hit, 128, 0)[3], 255);
        // The plate, replaced in place, keeps its square corners.
        assert_eq!(at(&icons[1].3, 0, 0), [200, 10, 10, 255]);
    }

    #[test]
    fn the_players_folder_wins_and_odd_files_are_left_alone() {
        let dir = std::env::temp_dir().join(format!("touch-icons-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        // A real PNG: 2 x 1 pixels, red and blue.
        let mut png = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut png, 2, 1);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            let mut w = enc.write_header().unwrap();
            w.write_image_data(&[255, 0, 0, 255, 0, 0, 255, 255])
                .unwrap();
        }
        std::fs::write(dir.join("jump.png"), &png).unwrap();
        std::fs::write(dir.join("notes.txt"), b"x").unwrap();
        std::fs::write(dir.join("Bad Name.png"), &png).unwrap();
        std::fs::write(dir.join("broken.png"), b"not a png").unwrap();
        let icons = user_icons(&dir);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(icons.len(), 1);
        assert_eq!(icons[0].0, "touch/user/jump");
        // 2 x 1 enlarged: 256 wide, 128 high, centred.
        assert_eq!(at(&icons[0].3, 10, 128), [255, 0, 0, 255]);
        assert_eq!(at(&icons[0].3, 245, 128), [0, 0, 255, 255]);
        assert_eq!(at(&icons[0].3, 10, 10)[3], 0);
    }
}
