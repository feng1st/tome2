//! The 16 ToME terminal colors (from src/variable.cc angband_color_table),
//! with the `lib/pref/colors.prf` redefinitions applied (init1.cc load_prefs).

use bevy::prelude::Color;
use std::sync::OnceLock;

pub const PALETTE: [[u8; 3]; 16] = [
    [0x00, 0x00, 0x00], // 0  d  dark
    [0xFF, 0xFF, 0xFF], // 1  w  white
    [0x80, 0x80, 0x80], // 2  s  slate
    [0xFF, 0x80, 0x00], // 3  o  orange
    [0xC0, 0x00, 0x00], // 4  r  red
    [0x00, 0x80, 0x40], // 5  g  green
    [0x00, 0x00, 0xFF], // 6  b  blue
    [0x80, 0x40, 0x00], // 7  u  umber
    [0x40, 0x40, 0x40], // 8  D  light dark
    [0xC0, 0xC0, 0xC0], // 9  W  light white
    [0xFF, 0x00, 0xFF], // 10 v  violet
    [0xFF, 0xFF, 0x00], // 11 y  yellow
    [0xFF, 0x00, 0x00], // 12 R  light red
    [0x00, 0xFF, 0x00], // 13 G  light green
    [0x00, 0xFF, 0xFF], // 14 B  light blue
    [0xC0, 0x80, 0x40], // 15 U  light umber
];

/// `color_char_to_attr` (init1.cc:508): a color letter to its TERM_*
/// index (the palette order above); unknown letters give -1.
pub fn color_char_to_attr(c: char) -> i32 {
    match c {
        'd' => 0,
        'w' => 1,
        's' => 2,
        'o' => 3,
        'r' => 4,
        'g' => 5,
        'b' => 6,
        'u' => 7,
        'D' => 8,
        'W' => 9,
        'v' => 10,
        'y' => 11,
        'R' => 12,
        'G' => 13,
        'B' => 14,
        'U' => 15,
        _ => -1,
    }
}

/// `mh_attr` (spells1.cc:1016): a legal multi-hued colour index.
/// `randint(max)` picks 1..=max; values past the table give white.
pub fn mh_attr(max: i32, rng: &mut impl rand::Rng) -> u8 {
    match crate::rng::randint(max, rng) {
        1 => 4,
        2 => 5,
        3 => 6,
        4 => 11,
        5 => 3,
        6 => 10,
        7 => 12,
        8 => 13,
        9 => 14,
        10 => 7,
        11 => 15,
        12 => 2,
        13 => 1,
        14 => 9,
        15 => 8,
        _ => 1,
    }
}

/// `colors.prf` V: lines (init1.cc process_pref_file_aux): each line
/// `V:<idx>:<flag>:<r>:<g>:<b>` overrides a palette entry.
pub fn parse_colors_prf(text: &str) -> [[u8; 3]; 16] {
    let mut p = PALETTE;
    for line in text.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("V:") else {
            continue;
        };
        let parts: Vec<&str> = rest.split(':').collect();
        if parts.len() < 5 {
            continue;
        }
        let hex = |s: &str| u8::from_str_radix(s.trim_start_matches("0x"), 16).unwrap_or(0);
        let idx = parts[0].trim().parse::<usize>().unwrap_or(16);
        if idx < 16 {
            p[idx] = [hex(parts[2]), hex(parts[3]), hex(parts[4])];
        }
    }
    p
}

static EFFECTIVE: OnceLock<[[u8; 3]; 16]> = OnceLock::new();

fn effective() -> &'static [[u8; 3]; 16] {
    EFFECTIVE.get_or_init(|| {
        parse_colors_prf(include_str!("../assets/data/colors.prf"))
    })
}

pub const DIM_FACTOR: f32 = 0.35;

pub fn palette(idx: u8) -> Color {
    let c = effective()[(idx as usize) & 15];
    Color::srgb_u8(c[0], c[1], c[2])
}

/// Same color, darkened -- used for "remembered" (explored but not visible) cells.
pub fn dimmed(idx: u8) -> Color {
    let c = effective()[(idx as usize) & 15];
    Color::srgb_u8(
        (c[0] as f32 * DIM_FACTOR) as u8,
        (c[1] as f32 * DIM_FACTOR) as u8,
        (c[2] as f32 * DIM_FACTOR) as u8,
    )
}

/// Background tile index used for lit floor cells.
pub const BG_FLOOR_LIT: usize = 8; // light dark
/// Background tile index for everything else (plain black).
pub const BG_DARK: usize = 0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors_prf_overrides_the_default_palette() {
        let p = parse_colors_prf("V:2:0x03:0x8C:0x8C:0x8C\nV:4:0x04:0xC9:0x00:0x00\n");
        assert_eq!(p[2], [0x8C, 0x8C, 0x8C]);
        assert_eq!(p[4], [0xC9, 0x00, 0x00]);
        assert_eq!(p[0], PALETTE[0], "untouched entries keep the default");
        // the shipped asset is actually applied by `palette`
        assert_eq!(palette(2), Color::srgb_u8(0x8C, 0x8C, 0x8C));
        assert_eq!(palette(3), Color::srgb_u8(0xFF, 0x77, 0x00));
    }

    #[test]
    fn color_letters_map_to_the_palette_order() {
        assert_eq!(color_char_to_attr('d'), 0);
        assert_eq!(color_char_to_attr('w'), 1);
        assert_eq!(color_char_to_attr('s'), 2);
        assert_eq!(color_char_to_attr('u'), 7);
        assert_eq!(color_char_to_attr('D'), 8);
        assert_eq!(color_char_to_attr('U'), 15);
        assert_eq!(color_char_to_attr('?'), -1);
    }
}

#[cfg(test)]
mod mh_attr_tests {
    #[test]
    fn mh_attr_stays_in_the_table() {
        let mut rng = crate::rng::new_seeded_rng(3);
        for _ in 0..200 {
            assert!(super::mh_attr(15, &mut rng) < 16);
        }
        // randint(0) is 1, so small maxima still yield a legal colour.
        assert!(super::mh_attr(0, &mut rng) < 16);
    }
}
