//! The character notes file (src/notes.cc).
//!
//! Append-only log at `<crate root>/notes.txt`.  Every entry is a single
//! line in the original format:
//!
//! ```text
//! Turn        1234   Town   : some text
//! Turn        9999 Lev 25 L: Reached level 26
//! ```
//!
//! (`"%-20s %s %c: %s"` where the depth field is `"  Town"` or `"Lev%3d"`.)
//! Type notes (birth / winner / save / new session) are whole blocks printed
//! by `add_note_type` (notes.cc:102).

use bevy::prelude::*;
use std::io::Write;
use std::path::PathBuf;

/// The note code letters used by the original (`add_note(note, code)`).
pub const NOTE_USER: char = ' ';
pub const NOTE_LEVEL: char = 'L';
pub const NOTE_UNIQUE: char = 'U';
pub const NOTE_ARTIFACT: char = 'A';
/// Quest notes use the same code as a plain user note in base (there is
/// no dedicated call site); kept for the note-code inventory.
#[allow(dead_code)]
pub const NOTE_QUEST: char = 'Q';

/// A pending `do_cmd_note` text prompt (cmd4.cc:2602).
#[derive(Resource)]
pub struct Notes {
    pub path: PathBuf,
    /// Notes are always appended (the original has no switch); kept for
    /// symmetry with the save flag.
    pub enabled: bool,
    /// `Some` while the player is typing a note; Esc cancels, Enter saves.
    pub input: Option<String>,
    /// Last level seen, for the "Reached level N" hook (xtra2.cc:1950).
    pub last_level: u32,
    /// The winner note has been written (notes.cc NOTE_WINNER).
    pub winner_noted: bool,
}

impl Default for Notes {
    fn default() -> Self {
        Notes {
            path: default_path(),
            enabled: true,
            input: None,
            last_level: 1,
            winner_noted: false,
        }
    }
}

/// The notes file lives next to the save game (crate root).
pub fn default_path() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/notes.txt"))
}

/// `show_notes_file` (notes.cc:37): the note file as shown by `show_file`,
/// i.e. a `Note file <name>` caption followed by the file contents in
/// file order.  Returns None when the file does not exist.
pub fn show_notes_file(path: &std::path::Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let name = path
        .file_name()
        .map(|f| f.to_string_lossy().into_owned())
        .unwrap_or_default();
    Some(format!("Note file {}\n\n{}", name, text))
}

/// `note_path`/`output_note` (notes.cc:27/52): append one formatted line
/// to the notes file.  I/O errors are ignored exactly like the original.
pub fn output_note(path: &std::path::Path, line: &str) {
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(f, "{}", line);
    }
}

/// `add_note` (notes.cc:73): `"% -20s %s %c: %s"`, depth `"  Town"` or
/// `"Lev%3d"`, text clipped to 60 chars.
pub fn format_note(turn: u64, depth: u32, code: char, note: &str) -> String {
    let mut text: String = note.chars().take(60).collect();
    // The original `strncpy` copies up to 60 chars and stops at a NUL.
    if let Some(i) = text.find('\0') {
        text.truncate(i);
    }
    let depths = if depth == 0 {
        "  Town".to_string()
    } else {
        format!("Lev{:3}", depth)
    };
    format!("Turn {:>12} {} {}: {}", turn, depths, code, text)
}

impl Notes {
    /// `add_note(note, code)` (notes.cc:73).
    pub fn add_note(&self, code: char, note: &str, turn: u64, depth: u32) {
        if !self.enabled {
            return;
        }
        output_note(&self.path, &format_note(turn, depth, code, note));
    }

    /// `xtra2.cc:1950`: "Reached level N".
    pub fn add_level(&self, level: u32, turn: u64, depth: u32) {
        self.add_note(NOTE_LEVEL, &format!("Reached level {}", level), turn, depth);
    }

    /// `xtra2.cc:3213`: "Killed <unique>".
    pub fn add_unique(&self, name: &str, turn: u64, depth: u32) {
        self.add_note(NOTE_UNIQUE, &format!("Killed {}", name), turn, depth);
    }

    /// `spells2.cc:2074 note_found_object`: "Found The <artifact>".
    pub fn add_artifact(&self, name: &str, turn: u64, depth: u32) {
        self.add_note(NOTE_ARTIFACT, &format!("Found The {}", name), turn, depth);
    }

    /// Detects level-ups that happened since the last call (the port has no
    /// central `check_experience` hook we own, so this is polled once per
    /// player input frame).  Returns the levels gained.
    pub fn check_level_up(&mut self, level: u32, turn: u64, depth: u32) -> u32 {
        if level > self.last_level {
            for l in (self.last_level + 1)..=level {
                self.add_level(l, turn, depth);
            }
            let gained = level - self.last_level;
            self.last_level = level;
            return gained;
        }
        self.last_level = level;
        0
    }

    /// `add_note_type(NOTE_BIRTH)` (notes.cc:116).  The block itself ends
    /// with a newline, and `output_note` adds another, so the closing rule
    /// is followed by a blank line exactly like the original.
    pub fn add_birth(&self, player: &str, race_class: &str, timestamp: &str) {
        let block = format!(
            "\n================================================\n{} the {}\nBorn on {}\n================================================\n",
            player, race_class, timestamp
        );
        self.raw(&block);
    }

    /// `add_note_type(NOTE_WINNER)` (notes.cc:140).
    pub fn add_winner(&self, player: &str, timestamp: &str) {
        let block = format!(
            "{} slew Morgoth on {}\nLong live {}!\n================================================",
            player, timestamp, player
        );
        self.raw(&block);
    }

    /// `add_note_type(NOTE_SAVE_GAME)` (notes.cc:151).
    pub fn add_save_game(&self, timestamp: &str) {
        self.raw(&format!("\nSession end: {}", timestamp));
    }

    /// `add_note_type(NOTE_ENTER_DUNGEON)` (notes.cc:159); like the birth
    /// block the original's string ends with a newline.
    pub fn add_enter_dungeon(&self, timestamp: &str) {
        self.raw(&format!(
            "================================================\nNew session start: {}\n",
            timestamp
        ));
    }

    fn raw(&self, text: &str) {
        if !self.enabled {
            return;
        }
        output_note(&self.path, text);
    }
}

/// The local timestamp of the original (`strftime "%Y-%m-%d at %H:%M:%S"`,
/// notes.cc:110).  `localtime()` — not `gmtime()` — is what the original
/// uses, so the port applies the local UTC offset (DST included).
pub fn timestamp_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format_timestamp_at(secs, local_offset_secs(secs))
}

/// The local-time UTC offset (seconds east of Greenwich) at `secs`;
/// `localtime_r`'s tm_gmtoff, 0 when unavailable (non-unix).
#[cfg(unix)]
fn local_offset_secs(secs: u64) -> i64 {
    unsafe {
        let t = secs as libc::time_t;
        let mut tm: libc::tm = std::mem::zeroed();
        if libc::localtime_r(&t, &mut tm).is_null() {
            return 0;
        }
        tm.tm_gmtoff
    }
}

#[cfg(not(unix))]
fn local_offset_secs(_secs: u64) -> i64 {
    0
}

/// Format epoch seconds as `%Y-%m-%d at %H:%M:%S` in UTC.
pub fn format_timestamp(secs: u64) -> String {
    format_timestamp_at(secs, 0)
}

/// Format epoch seconds shifted by `offset` seconds (east positive) as
/// `%Y-%m-%d at %H:%M:%S`.
pub fn format_timestamp_at(secs: u64, offset: i64) -> String {
    let secs = (secs as i64 + offset).max(0) as u64;
    let days = (secs / 86400) as i64;
    let rem = secs % 86400;
    let (h, mi, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    // Howard Hinnant's civil_from_days.
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{:04}-{:02}-{:02} at {:02}:{:02}:{:02}", y, m, d, h, mi, s)
}

/// On entering the game: a fresh character writes the birth block
/// (birth.cc:2732 NOTE_BIRTH); a continued save writes the new-session
/// block (dungeon.cc:4734 NOTE_ENTER_DUNGEON).  Either way the level-up
/// tracker starts at the current level.
pub fn track_level_on_enter(
    ps: Res<crate::game::PlayerState>,
    pending: Option<Res<crate::save::PendingLoad>>,
    mut notes: ResMut<Notes>,
) {
    notes.last_level = ps.level;
    if pending.is_some() {
        notes.add_enter_dungeon(&timestamp_now());
    } else {
        let race_class = format!("{} {}", ps.race_name, ps.class_name);
        notes.add_birth(&ps.name, &race_class, &timestamp_now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_line_matches_original_layout() {
        // notes.cc:85/88/92: "Turn % 12ld" + "  Town"/"Lev%3d" + code.
        let line = format_note(1234, 0, NOTE_USER, "hello world");
        assert_eq!(line, "Turn         1234   Town  : hello world");
        let line = format_note(9999999, 25, NOTE_LEVEL, "Reached level 26");
        assert_eq!(line, "Turn      9999999 Lev 25 L: Reached level 26");
    }

    #[test]
    fn note_text_is_clipped_to_60_chars() {
        let long = "x".repeat(100);
        let line = format_note(1, 1, NOTE_USER, &long);
        let tail = line.split(": ").nth(1).unwrap();
        assert_eq!(tail.len(), 60);
    }

    #[test]
    fn append_only_file_grows_per_note() {
        let dir = std::env::temp_dir().join(format!("tome-notes-{}", std::process::id()));
        let path = dir.join("notes.txt");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let notes = Notes {
            path: path.clone(),
            enabled: true,
            input: None,
            last_level: 1,
            winner_noted: false,
        };
        notes.add_note(NOTE_USER, "first", 10, 0);
        notes.add_note(NOTE_UNIQUE, "Killed Gothmog", 20, 5);
        notes.add_artifact("Ring of Power", 30, 6);
        let text = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].ends_with("  : first"));
        assert_eq!(lines[1], "Turn           20 Lev  5 U: Killed Gothmog");
        assert_eq!(
            lines[2],
            "Turn           30 Lev  6 A: Found The Ring of Power"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn note_type_blocks_match_the_original() {
        let dir = std::env::temp_dir().join(format!("tome-notes-types-{}", std::process::id()));
        let path = dir.join("notes.txt");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let notes = Notes {
            path: path.clone(),
            enabled: true,
            input: None,
            last_level: 1,
            winner_noted: false,
        };
        notes.add_birth("Frodo", "Hobbit Rogue", "2026-09-17 at 12:00:00");
        notes.add_winner("Frodo", "2026-09-18 at 01:00:00");
        notes.add_save_game("2026-09-18 at 02:00:00");
        notes.add_enter_dungeon("2026-09-18 at 03:00:00");
        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(
            text,
            "\n================================================\n\
             Frodo the Hobbit Rogue\n\
             Born on 2026-09-17 at 12:00:00\n\
             ================================================\n\n\
             Frodo slew Morgoth on 2026-09-18 at 01:00:00\n\
             Long live Frodo!\n\
             ================================================\n\
             \nSession end: 2026-09-18 at 02:00:00\n\
             ================================================\n\
             New session start: 2026-09-18 at 03:00:00\n\n"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn level_up_detection_writes_each_gained_level() {
        let dir = std::env::temp_dir().join(format!("tome-notes-lvl-{}", std::process::id()));
        let path = dir.join("notes.txt");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut notes = Notes {
            path: path.clone(),
            enabled: true,
            input: None,
            last_level: 1,
            winner_noted: false,
        };
        assert_eq!(notes.check_level_up(1, 0, 0), 0);
        assert_eq!(notes.check_level_up(3, 100, 4), 2);
        assert_eq!(notes.check_level_up(3, 200, 5), 0);
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("Reached level 2"));
        assert!(text.contains("Reached level 3"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn timestamp_conversion() {
        // 2001-09-09 01:46:40 UTC
        assert_eq!(format_timestamp(1_000_000_000), "2001-09-09 at 01:46:40");
        assert_eq!(format_timestamp(0), "1970-01-01 at 00:00:00");
    }

    /// notes.cc:110 uses localtime(); the formatter must shift by the
    /// local UTC offset (and the offset source must be localtime_r).
    #[cfg(unix)]
    #[test]
    fn timestamp_now_uses_localtime_not_utc() {
        let secs = 1_000_000_000u64;
        let offset = local_offset_secs(secs);
        // Cross-check the offset against libc's own localtime breakdown.
        let (y, mo, d, h, mi, s) = unsafe {
            let t = secs as libc::time_t;
            let mut tm: libc::tm = std::mem::zeroed();
            assert!(!libc::localtime_r(&t, &mut tm).is_null());
            (
                tm.tm_year + 1900,
                tm.tm_mon + 1,
                tm.tm_mday,
                tm.tm_hour,
                tm.tm_min,
                tm.tm_sec,
            )
        };
        assert_eq!(
            format_timestamp_at(secs, offset),
            format!("{:04}-{:02}-{:02} at {:02}:{:02}:{:02}", y, mo, d, h, mi, s)
        );
        // A non-zero offset really moves the stamp away from the UTC one.
        if offset != 0 {
            assert_ne!(format_timestamp_at(secs, offset), format_timestamp(secs));
        }
        assert_eq!(format_timestamp_at(secs, -5 * 3600), "2001-09-08 at 20:46:40");
    }

    #[test]
    fn show_notes_file_prefixes_the_caption() {
        let dir = std::env::temp_dir().join(format!("tome-notes-show-{}", std::process::id()));
        let path = dir.join("Hero.txt");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&path, "Turn 1 Le  1: first\n").unwrap();
        let shown = show_notes_file(&path).unwrap();
        assert!(shown.starts_with("Note file Hero.txt\n\n"));
        assert!(shown.contains("Turn 1 Le  1: first"));
        let missing = dir.join("nope.txt");
        assert!(show_notes_file(&missing).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
