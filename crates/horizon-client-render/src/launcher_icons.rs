//! The phone launcher's icons (`launcher_screen.rs` in horizon-client, docs/unification/launcher.md):
//! the design system's glass set (`crates/horizon-theme/icons/glass/`, Hugeicons, MIT), white on
//! clear, rasterized at [`SIDE`] pixels from those SVGs into `assets/launcher/`. The boot screen's
//! atlas (`HudAtlas::boot`) keeps them in its last rows as `launcher/<name>`, tinted when drawn.

/// The side of each icon, in pixels.
pub const SIDE: u32 = 64;

/// The icons by the launcher's name for them, with the glass icon each comes from.
pub const ICONS: &[(&str, &[u8])] = &[
    ("folder", include_bytes!("../assets/launcher/folder-01.png")),
    (
        "controls",
        include_bytes!("../assets/launcher/game-controller-01.png"),
    ),
    ("mods", include_bytes!("../assets/launcher/puzzle.png")),
    (
        "edit",
        include_bytes!("../assets/launcher/pencil-edit-02.png"),
    ),
    ("user", include_bytes!("../assets/launcher/user.png")),
    (
        "accounts",
        include_bytes!("../assets/launcher/user-multiple-02.png"),
    ),
    (
        "settings",
        include_bytes!("../assets/launcher/sliders-horizontal.png"),
    ),
    ("refresh", include_bytes!("../assets/launcher/refresh.png")),
    ("delete", include_bytes!("../assets/launcher/delete-02.png")),
    (
        "info",
        include_bytes!("../assets/launcher/information-circle.png"),
    ),
    ("clock", include_bytes!("../assets/launcher/clock.png")),
    ("play", include_bytes!("../assets/launcher/play.png")),
    ("add", include_bytes!("../assets/launcher/plus-sign.png")),
    (
        "next",
        include_bytes!("../assets/launcher/arrow-right-01.png"),
    ),
    // arrow-right-01 mirrored: the set draws no left arrow.
    (
        "back",
        include_bytes!("../assets/launcher/arrow-left-01.png"),
    ),
    ("share", include_bytes!("../assets/launcher/upload-04.png")),
    (
        "profile",
        include_bytes!("../assets/launcher/package-01.png"),
    ),
    (
        "graphics",
        include_bytes!("../assets/launcher/computer.png"),
    ),
    ("fps", include_bytes!("../assets/launcher/flash.png")),
    (
        "resolution",
        include_bytes!("../assets/launcher/dashboard-square-01.png"),
    ),
    (
        "language",
        include_bytes!("../assets/launcher/globe-02.png"),
    ),
    ("check", include_bytes!("../assets/launcher/tick-02.png")),
    ("close", include_bytes!("../assets/launcher/cancel-01.png")),
    (
        "expand",
        include_bytes!("../assets/launcher/arrow-down-01.png"),
    ),
    (
        "download",
        include_bytes!("../assets/launcher/download-04.png"),
    ),
    ("link", include_bytes!("../assets/launcher/link-01.png")),
    ("github", include_bytes!("../assets/launcher/github.png")),
];

/// The rows the icons take at the foot of an atlas `width` texels wide.
pub fn rows(width: u32) -> u32 {
    let per_row = (width / SIDE).max(1) as usize;
    ICONS.len().div_ceil(per_row) as u32 * SIDE
}

/// Copies the icons into the last [`rows`] of an RGBA atlas `width` by `height`, returning each
/// one's name and texel rectangle (x, y, w, h). An icon that doesn't decode is left out.
pub fn place(pixels: &mut [u8], width: u32, height: u32) -> Vec<(String, [u32; 4])> {
    let per_row = (width / SIDE).max(1);
    let top = height.saturating_sub(rows(width));
    let mut placed = Vec::new();
    for (i, (name, png)) in ICONS.iter().enumerate() {
        let Ok((w, h, rgba)) = crate::hud::decode_bytes(png) else {
            continue;
        };
        if w != SIDE || h != SIDE {
            continue;
        }
        let (x0, y0) = (
            (i as u32 % per_row) * SIDE,
            top + (i as u32 / per_row) * SIDE,
        );
        for row in 0..SIDE {
            let from = (row * SIDE * 4) as usize;
            let to = (((y0 + row) * width + x0) * 4) as usize;
            pixels[to..to + (SIDE * 4) as usize]
                .copy_from_slice(&rgba[from..from + (SIDE * 4) as usize]);
        }
        placed.push((format!("launcher/{name}"), [x0, y0, SIDE, SIDE]));
    }
    placed
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_icon_decodes_at_its_side_and_fits_its_rows() {
        let (width, height) = (1024, 1024);
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        let placed = super::place(&mut pixels, width, height);
        assert_eq!(placed.len(), super::ICONS.len());
        let top = height - super::rows(width);
        assert!(
            placed
                .iter()
                .all(|(_, [_, y, _, h])| *y >= top && y + h <= height)
        );
        // Something white landed where the first icon went.
        let [x, y, w, h] = placed[0].1;
        let lit = (y..y + h)
            .flat_map(|r| (x..x + w).map(move |c| (r, c)))
            .any(|(r, c)| pixels[((r * width + c) * 4 + 3) as usize] > 0);
        assert!(lit);
    }
}
