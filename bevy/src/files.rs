//! Port of the non-UI parts of `src/files.cc` (the character-sheet
//! display helpers and small data utilities; the pref-file parser and
//! help/screenshot viewers have no Bevy counterpart).

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::data::{DungeonDef, GameData};
use crate::game::{PlayerState, PlotQuest, Wilderness};

/// `likert` (files.cc:1259): a rating word for `x` relative to `y`.
/// The original also sets `likert_color`; the text is what is ported.
pub fn likert(x: i32, y: i32) -> String {
    let y = y.max(1);
    if x < 0 {
        return "Very Bad".to_string();
    }
    match x / y {
        0 | 1 => "Bad".to_string(),
        2 => "Poor".to_string(),
        3 | 4 => "Fair".to_string(),
        5 => "Good".to_string(),
        6 => "Very Good".to_string(),
        7 | 8 => "Excellent".to_string(),
        9..=13 => "Superb".to_string(),
        14..=17 => "Heroic".to_string(),
        n => format!("Legendary[{}]", ((n - 17) * 5) / 2),
    }
}

/// `number_to_digit` (files.cc:1786): `|n|` as a digit, `*` past 9.
pub fn number_to_digit(n: i32) -> char {
    let n = n.abs();
    if n > 9 {
        '*'
    } else {
        (b'0' + n as u8) as char
    }
}

/// `object_flag_cell` (files.cc:1830): one cell of the character-sheet
/// object-flag grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObjectFlagCell {
    /// '\0' empty, 'n' numeric, 'b' boolean, '+' resist, '*' immunity,
    /// 'f' fixed value.
    pub kind: char,
    pub pval: i32,
}

pub const OBJECT_FLAG_CELL_EMPTY: ObjectFlagCell = ObjectFlagCell {
    kind: '\0',
    pval: 0,
};

/// `object_flag_types_are_compatible` (files.cc:1797).
pub fn object_flag_types_are_compatible(type_a: char, type_b: char) -> bool {
    match type_a {
        'n' => type_b == 'n',
        'b' => type_b == 'b',
        '+' | '*' => type_b == '+' || type_b == '*',
        'f' => type_b == 'f',
        '\0' => true,
        _ => false,
    }
}

/// `monoid` (monoid.hpp:16): a type with an associative `append` and an
/// identity `empty`.  The one original use is `object_flag_cell` in
/// `display_flag_row` (files.cc:2002), with `object_flag_cell_append`
/// and `OBJECT_FLAG_CELL_EMPTY` as its instance.
pub fn mconcat<T: Copy>(xs: &[T], empty: T, append: impl Fn(T, T) -> T) -> T {
    xs.iter().fold(empty, |acc, &x| append(acc, x))
}

/// `object_flag_cell_append` (files.cc:1864).
pub fn object_flag_cell_append(a: ObjectFlagCell, b: ObjectFlagCell) -> ObjectFlagCell {
    if a.kind == '\0' {
        return b;
    }
    if b.kind == '\0' {
        return a;
    }
    assert!(
        object_flag_types_are_compatible(a.kind, b.kind),
        "incompatible object flag cells"
    );
    if b.kind == 'b' {
        return b;
    }
    if b.kind == 'n' || a.kind == 'f' {
        return ObjectFlagCell {
            kind: a.kind,
            pval: a.pval + b.pval,
        };
    }
    if a.kind == '*' {
        return ObjectFlagCell {
            kind: '*',
            pval: 0,
        };
    }
    if b.kind == '*' {
        return ObjectFlagCell {
            kind: '*',
            pval: 0,
        };
    }
    ObjectFlagCell {
        kind: '+',
        pval: 0,
    }
}

/// `object_flag_cell_to_char` (files.cc:1944): the glyph and sign of a
/// finished cell.
pub fn object_flag_cell_to_char(cell: ObjectFlagCell) -> (char, i32) {
    match cell.kind {
        'n' | 'f' => {
            if cell.pval == 0 {
                ('+', 1)
            } else {
                (
                    number_to_digit(cell.pval),
                    if cell.pval >= 0 { 1 } else { -1 },
                )
            }
        }
        'b' | '+' => ('+', 1),
        '*' => ('*', 1),
        _ => ('.', 0),
    }
}

/// `get_line` (files.cc:5153): read the `line`-th line of a file (0-based
/// in the port; the original is 1-based).
pub fn get_line(path: &Path, line: usize) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    text.lines().nth(line).map(|s| s.to_string())
}

/// The timed-autosave test from `dungeon.cc:1075` (the original uses the
/// turn counter and `autosave_freq * 10`).
pub fn timed_autosave_due(turn: u64, freq: u32, enabled: bool) -> bool {
    enabled && freq > 0 && turn % (freq as u64 * 10) == 0
}

// --- File-name helpers (files.cc:75-111) ----------------------------------

/// `name_file_note` (files.cc:75): the character's `.nte` note file.
#[allow(dead_code)]
pub fn name_file_note(sv: &str) -> String {
    format!("{}{}", sv, ".nte")
}

/// `name_file_pref` (files.cc:84): a `.prf` preference file.
#[allow(dead_code)]
pub fn name_file_pref(sv: &str) -> String {
    format!("{}{}", sv, ".prf")
}

/// The save directory (`ANGBAND_DIR_SAVE`): the port keeps its single
/// save slot in the crate root (`save::path()`).
#[allow(dead_code)]
pub fn save_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `name_file_save()` (files.cc:93): the current player's save slot.
/// The port has a single named slot (`save::path()`), so the original's
/// `player_base`-derived filename becomes that fixed path.
#[allow(dead_code)]
pub fn name_file_save() -> PathBuf {
    PathBuf::from(crate::save::path())
}

/// `name_file_save(sv)` (files.cc:98): `ANGBAND_DIR_SAVE/<sv>`.
#[allow(dead_code)]
pub fn name_file_save_named(sv: &str) -> PathBuf {
    save_dir().join(sv)
}

/// `name_file_dungeon_save(ext)` (files.cc:107): the player save with the
/// level extension swapped in (`fs::path::replace_extension`).
#[allow(dead_code)]
pub fn name_file_dungeon_save(ext: &str) -> PathBuf {
    let mut p = name_file_save();
    p.set_extension(ext);
    p
}

// --- Pref-file parsing (files.cc:129-1027) --------------------------------

/// `tokenize` (files.cc:129).  The original mutates the buffer in place
/// and returns pointers into it; the port builds owned tokens instead.
/// The quote formalism (`'c`, `'\c`) and the in-place repair of a missing
/// closing quote are reproduced, as is the "last token keeps delimiters"
/// rule.
#[allow(dead_code)]
pub fn tokenize(buf: &str, num: usize, delim1: char, delim2: char) -> Vec<String> {
    let mut b: Vec<char> = buf.chars().collect();
    let mut tokens = Vec::new();
    let mut start = 0usize;
    while (tokens.len() as i64) < num as i64 - 1 {
        let mut t = start;
        while t < b.len() {
            let c = b[t];
            if c == delim1 || c == delim2 {
                break;
            }
            if c == '\'' {
                t += 1;
                if t < b.len() && b[t] == '\\' {
                    t += 1;
                }
                if t >= b.len() {
                    break;
                }
                t += 1;
                if t >= b.len() {
                    // The original writes the quote over the terminator.
                    b.push('\'');
                } else if b[t] != '\'' {
                    b[t] = '\'';
                }
            }
            if t < b.len() && b[t] == '\\' {
                t += 1;
            }
            t += 1;
        }
        if t >= b.len() {
            break;
        }
        tokens.push(b[start..t].iter().collect());
        start = t + 1;
    }
    tokens.push(b[start.min(b.len())..].iter().collect());
    tokens
}

/// The system name `$SYS` expands to (`ANGBAND_SYS`).  The original bakes
/// the platform in at build time; the Bevy port is its own "system".
pub const ANGBAND_SYS: &str = "bevy";

/// `process_pref_file_expr` (files.cc:749): evaluate one `?:` expression.
/// `sp` is the cursor into `buf`, `fp` receives the terminating character
/// (the original's `f`).  `skill_val` resolves a skill name to its value
/// (`find_skill_i` + `get_skill`); unknown skills leave the result "0".
#[allow(dead_code)]
pub fn process_pref_file_expr(
    buf: &[char],
    sp: &mut usize,
    fp: &mut char,
    skill_val: &mut dyn FnMut(&str) -> Option<i32>,
) -> String {
    let mut s = *sp;
    while s < buf.len() && buf[s].is_whitespace() {
        s += 1;
    }
    let b = s;
    let mut v = "?o?o?".to_string();

    if b < buf.len() && buf[b] == '[' {
        s += 1;
        let mut f = *fp;
        let t = process_pref_file_expr(buf, &mut s, &mut f, skill_val);
        if t.is_empty() {
            // Nothing.
        } else if t == "IOR" {
            v = "0".to_string();
            while s < buf.len() && f != ']' {
                let t = process_pref_file_expr(buf, &mut s, &mut f, skill_val);
                if !t.is_empty() && t != "0" {
                    v = "1".to_string();
                }
            }
        } else if t == "AND" {
            v = "1".to_string();
            while s < buf.len() && f != ']' {
                let t = process_pref_file_expr(buf, &mut s, &mut f, skill_val);
                if !t.is_empty() && t == "0" {
                    v = "0".to_string();
                }
            }
        } else if t == "NOT" {
            v = "1".to_string();
            while s < buf.len() && f != ']' {
                let t = process_pref_file_expr(buf, &mut s, &mut f, skill_val);
                if !t.is_empty() && t != "0" {
                    v = "0".to_string();
                }
            }
        } else if t == "EQU" {
            v = "1".to_string();
            let mut prev = t.clone();
            if s < buf.len() && f != ']' {
                prev = process_pref_file_expr(buf, &mut s, &mut f, skill_val);
            }
            while s < buf.len() && f != ']' {
                let t = process_pref_file_expr(buf, &mut s, &mut f, skill_val);
                if !t.is_empty() && prev != t {
                    v = "0".to_string();
                }
                prev = t;
            }
        } else if t == "SKILL" {
            v = "0".to_string();
            let mut skill: Option<i32> = None;
            while s < buf.len() && f != ']' {
                let t = process_pref_file_expr(buf, &mut s, &mut f, skill_val);
                if !t.is_empty() {
                    skill = skill_val(&t);
                }
            }
            if let Some(val) = skill {
                v = val.to_string();
            }
        } else {
            // Unknown function: consume the arguments.
            while s < buf.len() && f != ']' {
                let _ = process_pref_file_expr(buf, &mut s, &mut f, skill_val);
            }
        }

        if f != ']' {
            v = "?x?x?".to_string();
        }
        let f = if s < buf.len() {
            let c = buf[s];
            s += 1;
            c
        } else {
            '\0'
        };
        *fp = f;
        *sp = s;
    } else {
        while s < buf.len()
            && buf[s].is_ascii_graphic()
            && buf[s] != '['
            && buf[s] != ']'
        {
            s += 1;
        }
        let f = if s < buf.len() { buf[s] } else { '\0' };
        let token: String = buf[b..s].iter().collect();
        if s < buf.len() {
            s += 1;
        }
        if token.starts_with('$') {
            if token[1..] == *"SYS" {
                v = ANGBAND_SYS.to_string();
            }
        } else {
            v = token;
        }
        *fp = f;
        *sp = s;
    }

    v
}

/// `process_pref_file` (files.cc:922): read `name` from the user dir with
/// a fallback to the pref dir, then process its lines: comments/blank
/// lines are skipped, `?:` sets a bypass flag, `%:` includes another file
/// recursively and everything else goes to `aux` (the caller's
/// `process_pref_file_aux`).  Returns the error code and the messages the
/// original prints on failure.
#[allow(dead_code)]
pub fn process_pref_file(
    name: &str,
    user_dir: &Path,
    pref_dir: &Path,
    aux: &mut dyn FnMut(&str) -> i32,
    skill_val: &mut dyn FnMut(&str) -> Option<i32>,
) -> (i32, Vec<String>) {
    let text = std::fs::read_to_string(user_dir.join(name))
        .or_else(|_| std::fs::read_to_string(pref_dir.join(name)));
    let Ok(text) = text else {
        return (-1, Vec::new());
    };

    let mut num: i64 = -1;
    let mut err = 0;
    let mut bypass = false;
    let mut messages = Vec::new();
    let mut buf = String::new();

    for line in text.lines() {
        num += 1;
        buf = line.to_string();

        if line.is_empty() {
            continue;
        }
        if line.chars().next().is_some_and(|c| c.is_whitespace()) {
            continue;
        }
        if line.starts_with('#') {
            continue;
        }

        if line.starts_with("?:") {
            let chars: Vec<char> = line.chars().collect();
            let mut pos = 2;
            let mut f = ' ';
            let v = process_pref_file_expr(&chars, &mut pos, &mut f, skill_val);
            bypass = v == "0";
            continue;
        }

        if bypass {
            continue;
        }

        if line.starts_with('%') {
            let include: String = line.chars().skip(2).collect();
            let _ = process_pref_file(&include, user_dir, pref_dir, aux, skill_val);
            continue;
        }

        err = aux(line);
        if err != 0 {
            break;
        }
    }

    if err != 0 {
        messages.push(format!(
            "Error {} in line {} of file '{}'.",
            err, num, name
        ));
        messages.push(format!("Parsing '{}'", buf));
    }

    (err, messages)
}

// --- Terminal colour letters (init1.cc:553) -------------------------------

/// `conv_color` (init1.cc:553): attr index -> colour letter (the inverse
/// of `colors::color_char_to_attr`).
#[allow(dead_code)]
pub fn conv_color(attr: u8) -> char {
    const TABLE: [char; 16] = [
        'd', 'w', 's', 'o', 'r', 'g', 'b', 'u', 'D', 'W', 'v', 'y', 'R', 'G', 'B', 'U',
    ];
    TABLE[(attr & 0xf) as usize]
}

// --- Screenshots (files.cc:3479-3670) -------------------------------------

/// `cmovie_clean_line` (files.cc:3479) without the terminal reads: every
/// screen column yields the colour letter and the character to record.
/// `map_cell` returns the dungeon-map cell for map-area columns (the
/// original's `map_info_default`), `term_cell` the terminal contents.
#[allow(dead_code)]
pub fn cmovie_clean_line(
    wid: usize,
    map_cell: &dyn Fn(usize) -> Option<(u8, char)>,
    term_cell: &dyn Fn(usize) -> (u8, char),
) -> (Vec<char>, Vec<char>) {
    let mut abuf = Vec::with_capacity(wid);
    let mut cbuf = Vec::with_capacity(wid);
    for x in 0..wid {
        if let Some((a, c)) = map_cell(x) {
            abuf.push(conv_color(a & 0xf));
            cbuf.push(if c == '\0' { ' ' } else { c });
        } else {
            let (a, c) = term_cell(x);
            abuf.push(conv_color(a & 0xf));
            cbuf.push(c);
        }
    }
    (abuf, cbuf)
}

/// `help_file_screenshot` (files.cc:3537): each screen row as the
/// `&&&&&` marker followed by the raw attr/char pairs.
#[allow(dead_code)]
pub fn help_file_screenshot_text(rows: &[(Vec<char>, Vec<char>)]) -> String {
    let mut out = String::new();
    for (a, c) in rows {
        out.push_str("&&&&&");
        for (ca, cc) in a.iter().zip(c.iter()) {
            out.push(*ca);
            out.push(*cc);
        }
        out.push('\n');
    }
    out
}

/// `html_screenshot` (files.cc:3590): the XHTML wrapper with per-run
/// colour spans and the three escaped characters (`<`, `>`, `&`).
#[allow(dead_code)]
pub fn html_screenshot_text(
    name: &str,
    version: &str,
    palette: &[[u8; 3]; 16],
    rows: &[(Vec<char>, Vec<char>)],
) -> String {
    let span = |a: usize| {
        format!(
            "<span style=\"color: #{:02X}{:02X}{:02X}\">",
            palette[a][0], palette[a][1], palette[a][2]
        )
    };
    let mut out = String::new();
    out.push_str(
        "<?xml version=\"1.0\" encoding=\"iso-8859-1\"?>\n\
         <!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Strict//EN\" \"DTD/xhtml1-strict.dtd\">\n\
         <html xmlns=\"http://www.w3.org/1999/xhtml\">\n\
         <head>\n",
    );
    out.push_str(&format!(
        "<meta name=\"GENERATOR\" content=\"{}\"/>\n",
        version
    ));
    out.push_str(&format!("<title>{}</title>\n", name));
    out.push_str(
        "</head>\n<body>\n\
         <pre style=\"color: #ffffff; background-color: #000000; font-family: monospace\">\n",
    );
    out.push_str(&span(crate::colors::color_char_to_attr('w') as usize));
    out.push('\n');

    let mut oa = crate::colors::color_char_to_attr('w');
    for (a, c) in rows {
        for (ca, cc) in a.iter().zip(c.iter()) {
            let a_idx = crate::colors::color_char_to_attr(*ca);
            if oa != a_idx {
                out.push_str("</span>");
                out.push_str(&span(a_idx.max(0) as usize));
                oa = a_idx;
            }
            match cc {
                '<' => out.push_str("&lt;"),
                '>' => out.push_str("&gt;"),
                '&' => out.push_str("&amp;"),
                _ => out.push(*cc),
            }
        }
        out.push('\n');
    }
    out.push_str("</span>\n</pre>\n</body>\n</html>\n");
    out
}

// --- Character-sheet dump helpers (files.cc:2376-2830) --------------------

/// `file_character_print_grid_check_row` (files.cc:2376): any cell from
/// column 12 on holds `+`, `*` or a digit.  The original's `strstr(buf+12,
/// ..)` runs past short buffers; the port checks the available suffix.
#[allow(dead_code)]
pub fn file_character_print_grid_check_row(buf: &str) -> bool {
    buf.chars()
        .skip(12)
        .any(|c| matches!(c, '+' | '*' | '1'..='9'))
}

/// `file_character_print_grid` (files.cc:2397) without the terminal: the
/// two 40-column halves of each screen row are passed in and the lines
/// the original would print are returned.
#[allow(dead_code)]
pub fn file_character_print_grid_lines(
    left: &[String],
    right: &[String],
    show_gaps: bool,
    show_legend: bool,
) -> Vec<String> {
    let blank = " ".repeat(40);
    let mut out = Vec::new();
    let mut y = if show_legend { 3 } else { 4 };
    while y < 23 {
        let row = left.get(y).map(String::as_str).unwrap_or("");
        if row != blank && (y == 3 || show_gaps || file_character_print_grid_check_row(row)) {
            out.push(format!("        {}", row));
        }
        y += 1;
    }
    for y in 4..23 {
        let row = right.get(y).map(String::as_str).unwrap_or("");
        if row != blank && (show_gaps || file_character_print_grid_check_row(row)) {
            out.push(format!("        {}", row));
        }
    }
    out
}

/// `file_character_print_item` (files.cc:2439): one `"<label>) <desc>"``
/// line (the object description and its extra lines come from
/// `object_desc` / `object_out_desc`).
#[allow(dead_code)]
pub fn file_character_print_item_text(label: char, desc: &str) -> String {
    format!("{}) {}\n", label, desc)
}

/// `file_character_print_store` (files.cc:2454): the store header plus
/// one labelled line per stock item, or nothing for an empty store.
#[allow(dead_code)]
pub fn file_character_print_store_text(
    store_name: &str,
    place_name: &str,
    items: &[String],
) -> String {
    if items.is_empty() {
        return String::new();
    }
    let mut out = format!("  [{} Inventory - {}]\n\n", store_name, place_name);
    for (i, item) in items.iter().enumerate() {
        out.push_str(&file_character_print_item_text(i2a(i % 24), item));
    }
    out.push_str("\n\n");
    out
}

/// `I2A` (z-util.cc): 0..25 -> 'a'..'z', 26..51 -> 'A'..'Z'.
fn i2a(n: usize) -> char {
    if n < 26 {
        (b'a' + n as u8) as char
    } else {
        (b'A' + (n - 26) as u8) as char
    }
}

/// `file_character_check_stores` (files.cc:2484): report whether this
/// (town, store) identity was not seen before, recording it.  The
/// original compares `store_type *`; the port keys on town + slot.
#[allow(dead_code)]
pub fn file_character_check_stores(
    seen_stores: &mut HashSet<(u32, u32)>,
    town: u32,
    store: u32,
) -> bool {
    seen_stores.insert((town, store))
}

/// `file_character` (files.cc:2556): the first line of the dump.  The
/// rest of the ~1300-line file writer is a terminal/screen dump (HUD);
/// the deterministic helpers it uses are ported above.
#[allow(dead_code)]
pub fn character_sheet_header(version: &str) -> String {
    format!("  [{} Character Sheet]\n\n", version)
}

// --- Object-flag metadata (files.cc:1712-1949) ----------------------------

/// One `object_flag_meta` record (src/object_flag_meta.hpp): the flag's
/// names, the character-sheet slot it is drawn in, its priority and its
/// character-sheet type.  The original also stores the flag's bit set;
/// the port identifies a flag by its edit-file name (`e_name`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ObjectFlagMeta {
    /// Bit name (`TR_*` / `ESP_*`, object_flag_list.hpp `name`).
    pub name: &'static str,
    /// Edit-file name; the port's flag strings (`e_name`).
    pub e_name: &'static str,
    /// Character-sheet label (NULL = not drawn).
    pub label: Option<&'static str>,
    /// Character-sheet page/column/row (-1 = not drawn).
    pub page: i32,
    pub column: i32,
    pub row: i32,
    /// 'n' numeric, 'b' binary, '+' resist, '*' immune, '1'..'3' fixed
    /// light radii (object_flag_meta.cc NUMERIC/BINARY/TERNARY/FIXED).
    pub kind: char,
    /// Priority when two flags share a character-sheet cell.
    pub priority: i32,
    /// Is the flag *described* using PVAL?
    pub is_pval: bool,
    /// Does the flag affect ESP?
    pub is_esp: bool,
}

/// `object_flags_meta` (object_flag_meta.cc:5): every flag in
/// object_flag_list.hpp order.
#[allow(dead_code)]
pub fn object_flags_meta() -> &'static [ObjectFlagMeta] {
    &OBJECT_FLAGS_META
}

/// `object_flags_esp` (object_flag_meta.cc:40): the union of every flag
/// whose `is_esp` is set.  The port has no flag bit sets, so the result
/// is the list of the flags' edit-file names.
#[allow(dead_code)]
pub fn object_flags_esp() -> Vec<&'static str> {
    OBJECT_FLAGS_META
        .iter()
        .filter(|m| m.is_esp)
        .map(|m| m.e_name)
        .collect()
}

/// `object_flag_list.hpp` expanded exactly as object_flag_meta.cc does:
/// (name, e_name, c_name, page, column, row, type, priority, is_pval,
/// is_esp).  `kind` is the expanded NUMERIC/BINARY/TERNARY/FIXED char.
#[allow(dead_code)]
pub const OBJECT_FLAGS_META: [ObjectFlagMeta; 156] = [
    ObjectFlagMeta { name: "TR_STR", e_name: "STR", label: Some("Add Str"), page: 0, column: 0, row: 0, kind: 'n', priority: 0, is_pval: true, is_esp: false },
    ObjectFlagMeta { name: "TR_INT", e_name: "INT", label: Some("Add Int"), page: 0, column: 0, row: 1, kind: 'n', priority: 0, is_pval: true, is_esp: false },
    ObjectFlagMeta { name: "TR_WIS", e_name: "WIS", label: Some("Add Wis"), page: 0, column: 0, row: 2, kind: 'n', priority: 0, is_pval: true, is_esp: false },
    ObjectFlagMeta { name: "TR_DEX", e_name: "DEX", label: Some("Add Dex"), page: 0, column: 0, row: 3, kind: 'n', priority: 0, is_pval: true, is_esp: false },
    ObjectFlagMeta { name: "TR_CON", e_name: "CON", label: Some("Add Con"), page: 0, column: 0, row: 4, kind: 'n', priority: 0, is_pval: true, is_esp: false },
    ObjectFlagMeta { name: "TR_CHR", e_name: "CHR", label: Some("Add Chr"), page: 0, column: 0, row: 5, kind: 'n', priority: 0, is_pval: true, is_esp: false },
    ObjectFlagMeta { name: "TR_MANA", e_name: "MANA", label: Some("Mul Mana"), page: 0, column: 0, row: 6, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_SPELL", e_name: "SPELL", label: Some("Mul SPower"), page: 0, column: 0, row: 7, kind: 'b', priority: 0, is_pval: true, is_esp: false },
    ObjectFlagMeta { name: "TR_STEALTH", e_name: "STEALTH", label: Some("Add Stea."), page: 0, column: 0, row: 8, kind: 'n', priority: 0, is_pval: true, is_esp: false },
    ObjectFlagMeta { name: "TR_INFRA", e_name: "INFRA", label: Some("Add Infra"), page: 0, column: 0, row: 9, kind: 'n', priority: 0, is_pval: true, is_esp: false },
    ObjectFlagMeta { name: "TR_TUNNEL", e_name: "TUNNEL", label: Some("Add Tun.."), page: 0, column: 0, row: 10, kind: 'n', priority: 0, is_pval: true, is_esp: false },
    ObjectFlagMeta { name: "TR_SPEED", e_name: "SPEED", label: Some("Add Speed"), page: 0, column: 0, row: 11, kind: 'n', priority: 0, is_pval: true, is_esp: false },
    ObjectFlagMeta { name: "TR_BLOWS", e_name: "BLOWS", label: Some("Add Blows"), page: 0, column: 0, row: 12, kind: 'n', priority: 0, is_pval: true, is_esp: false },
    ObjectFlagMeta { name: "TR_CHAOTIC", e_name: "CHAOTIC", label: Some("Chaotic"), page: 0, column: 0, row: 13, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_VAMPIRIC", e_name: "VAMPIRIC", label: Some("Vampiric"), page: 0, column: 0, row: 14, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_SLAY_ANIMAL", e_name: "SLAY_ANIMAL", label: Some("Slay Anim."), page: 0, column: 1, row: 0, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_SLAY_EVIL", e_name: "SLAY_EVIL", label: Some("Slay Evil"), page: 0, column: 1, row: 1, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_SLAY_UNDEAD", e_name: "SLAY_UNDEAD", label: Some("Slay Und."), page: 0, column: 1, row: 2, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_SLAY_DEMON", e_name: "SLAY_DEMON", label: Some("Slay Demon"), page: 0, column: 1, row: 3, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_SLAY_ORC", e_name: "SLAY_ORC", label: Some("Slay Orc"), page: 0, column: 1, row: 4, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_SLAY_TROLL", e_name: "SLAY_TROLL", label: Some("Slay Troll"), page: 0, column: 1, row: 5, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_SLAY_GIANT", e_name: "SLAY_GIANT", label: Some("Slay Giant"), page: 0, column: 1, row: 6, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_SLAY_DRAGON", e_name: "SLAY_DRAGON", label: Some("Slay Drag."), page: 0, column: 1, row: 7, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_KILL_DRAGON", e_name: "KILL_DRAGON", label: Some("Kill Drag."), page: 0, column: 1, row: 8, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_VORPAL", e_name: "VORPAL", label: Some("Sharpness"), page: 0, column: 1, row: 9, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_IMPACT", e_name: "IMPACT", label: Some("Impact"), page: 0, column: 1, row: 10, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_BRAND_POIS", e_name: "BRAND_POIS", label: Some("Poison Brd"), page: 0, column: 1, row: 11, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_BRAND_ACID", e_name: "BRAND_ACID", label: Some("Acid Brand"), page: 0, column: 1, row: 12, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_BRAND_ELEC", e_name: "BRAND_ELEC", label: Some("Elec Brand"), page: 0, column: 1, row: 13, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_BRAND_FIRE", e_name: "BRAND_FIRE", label: Some("Fire Brand"), page: 0, column: 1, row: 14, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_BRAND_COLD", e_name: "BRAND_COLD", label: Some("Cold Brand"), page: 0, column: 1, row: 15, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_SUST_STR", e_name: "SUST_STR", label: Some("Sust Str"), page: 1, column: 0, row: 0, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_SUST_INT", e_name: "SUST_INT", label: Some("Sust Int"), page: 1, column: 0, row: 1, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_SUST_WIS", e_name: "SUST_WIS", label: Some("Sust Wis"), page: 1, column: 0, row: 2, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_SUST_DEX", e_name: "SUST_DEX", label: Some("Sust Dex"), page: 1, column: 0, row: 3, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_SUST_CON", e_name: "SUST_CON", label: Some("Sust Con"), page: 1, column: 0, row: 4, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_SUST_CHR", e_name: "SUST_CHR", label: Some("Sust Chr"), page: 1, column: 0, row: 5, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_INVIS", e_name: "INVIS", label: Some("Invisible"), page: 1, column: 0, row: 6, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_LIFE", e_name: "LIFE", label: Some("Mul life"), page: 1, column: 0, row: 7, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_IM_ACID", e_name: "IM_ACID", label: Some("Imm Acid"), page: 1, column: 1, row: 0, kind: '*', priority: 1, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_IM_ELEC", e_name: "IM_ELEC", label: Some("Imm Elec"), page: 1, column: 1, row: 1, kind: '*', priority: 1, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_IM_FIRE", e_name: "IM_FIRE", label: Some("Imm Fire"), page: 1, column: 1, row: 2, kind: '*', priority: 1, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_IM_COLD", e_name: "IM_COLD", label: Some("Imm Cold"), page: 1, column: 1, row: 3, kind: '*', priority: 1, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_SENS_FIRE", e_name: "SENS_FIRE", label: Some("Sens Fire"), page: 1, column: 0, row: 12, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_REFLECT", e_name: "REFLECT", label: Some("Reflect"), page: 1, column: 0, row: 13, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_FREE_ACT", e_name: "FREE_ACT", label: Some("Free Act"), page: 1, column: 0, row: 14, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_HOLD_LIFE", e_name: "HOLD_LIFE", label: Some("Hold Life"), page: 1, column: 0, row: 15, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_RES_ACID", e_name: "RES_ACID", label: Some("Res Acid"), page: 1, column: 1, row: 0, kind: '+', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_RES_ELEC", e_name: "RES_ELEC", label: Some("Res Elec"), page: 1, column: 1, row: 1, kind: '+', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_RES_FIRE", e_name: "RES_FIRE", label: Some("Res Fire"), page: 1, column: 1, row: 2, kind: '+', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_RES_COLD", e_name: "RES_COLD", label: Some("Res Cold"), page: 1, column: 1, row: 3, kind: '+', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_RES_POIS", e_name: "RES_POIS", label: Some("Res Pois"), page: 1, column: 1, row: 4, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_RES_FEAR", e_name: "RES_FEAR", label: Some("Res Fear"), page: 1, column: 1, row: 5, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_RES_LITE", e_name: "RES_LITE", label: Some("Res Light"), page: 1, column: 1, row: 6, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_RES_DARK", e_name: "RES_DARK", label: Some("Res Dark"), page: 1, column: 1, row: 7, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_RES_BLIND", e_name: "RES_BLIND", label: Some("Res Blind"), page: 1, column: 1, row: 8, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_RES_CONF", e_name: "RES_CONF", label: Some("Res Conf"), page: 1, column: 1, row: 9, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_RES_SOUND", e_name: "RES_SOUND", label: Some("Res Sound"), page: 1, column: 1, row: 10, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_RES_SHARDS", e_name: "RES_SHARDS", label: Some("Res Shard"), page: 1, column: 1, row: 11, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_RES_NETHER", e_name: "RES_NETHER", label: Some("Res Neth"), page: 1, column: 1, row: 12, kind: '+', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_RES_NEXUS", e_name: "RES_NEXUS", label: Some("Res Nexus"), page: 1, column: 1, row: 13, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_RES_CHAOS", e_name: "RES_CHAOS", label: Some("Res Chaos"), page: 1, column: 1, row: 14, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_RES_DISEN", e_name: "RES_DISEN", label: Some("Res Disen"), page: 1, column: 1, row: 15, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_SH_FIRE", e_name: "SH_FIRE", label: Some("Aura Fire"), page: 2, column: 0, row: 0, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_SH_ELEC", e_name: "SH_ELEC", label: Some("Aura Elec"), page: 2, column: 0, row: 1, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_AUTO_CURSE", e_name: "AUTO_CURSE", label: Some("Auto Curse"), page: 2, column: 0, row: 2, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_DECAY", e_name: "DECAY", label: None, page: -1, column: -1, row: -1, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_NO_TELE", e_name: "NO_TELE", label: Some("NoTeleport"), page: 2, column: 0, row: 4, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_NO_MAGIC", e_name: "NO_MAGIC", label: Some("AntiMagic"), page: 2, column: 0, row: 5, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_WRAITH", e_name: "WRAITH", label: Some("WraithForm"), page: 2, column: 0, row: 6, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_TY_CURSE", e_name: "TY_CURSE", label: Some("EvilCurse"), page: 2, column: 0, row: 7, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_EASY_KNOW", e_name: "EASY_KNOW", label: None, page: -1, column: -1, row: -1, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_HIDE_TYPE", e_name: "HIDE_TYPE", label: None, page: -1, column: -1, row: -1, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_SHOW_MODS", e_name: "SHOW_MODS", label: None, page: -1, column: -1, row: -1, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_INSTA_ART", e_name: "INSTA_ART", label: None, page: -1, column: -1, row: -1, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_FEATHER", e_name: "FEATHER", label: Some("Levitate"), page: 2, column: 0, row: 12, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_LITE1", e_name: "LITE1", label: Some("Lite"), page: 2, column: 0, row: 13, kind: '1', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_SEE_INVIS", e_name: "SEE_INVIS", label: Some("See Invis"), page: 2, column: 0, row: 14, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_NORM_ART", e_name: "NORM_ART", label: None, page: -1, column: -1, row: -1, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_SLOW_DIGEST", e_name: "SLOW_DIGEST", label: Some("Digestion"), page: 2, column: 1, row: 0, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_REGEN", e_name: "REGEN", label: Some("Regen"), page: 2, column: 1, row: 1, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_XTRA_MIGHT", e_name: "XTRA_MIGHT", label: Some("Xtra Might"), page: 2, column: 1, row: 2, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_XTRA_SHOTS", e_name: "XTRA_SHOTS", label: Some("Xtra Shots"), page: 2, column: 1, row: 3, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_IGNORE_ACID", e_name: "IGNORE_ACID", label: None, page: -1, column: -1, row: -1, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_IGNORE_ELEC", e_name: "IGNORE_ELEC", label: None, page: -1, column: -1, row: -1, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_IGNORE_FIRE", e_name: "IGNORE_FIRE", label: None, page: -1, column: -1, row: -1, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_IGNORE_COLD", e_name: "IGNORE_COLD", label: None, page: -1, column: -1, row: -1, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_ACTIVATE", e_name: "ACTIVATE", label: Some("Activate"), page: 2, column: 1, row: 8, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_DRAIN_EXP", e_name: "DRAIN_EXP", label: Some("Drain Exp"), page: 2, column: 1, row: 9, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_TELEPORT", e_name: "TELEPORT", label: Some("Teleport"), page: 2, column: 1, row: 10, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_AGGRAVATE", e_name: "AGGRAVATE", label: Some("Aggravate"), page: 2, column: 1, row: 11, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_BLESSED", e_name: "BLESSED", label: Some("Blessed"), page: 2, column: 1, row: 12, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_CURSED", e_name: "CURSED", label: Some("Cursed"), page: 2, column: 1, row: 13, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_HEAVY_CURSE", e_name: "HEAVY_CURSE", label: Some("Hvy Curse"), page: 2, column: 1, row: 14, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_PERMA_CURSE", e_name: "PERMA_CURSE", label: Some("Prm Curse"), page: 2, column: 1, row: 15, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_NEVER_BLOW", e_name: "NEVER_BLOW", label: Some("No blows"), page: 3, column: 0, row: 0, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_PRECOGNITION", e_name: "PRECOGNITION", label: Some("Precogn."), page: 3, column: 0, row: 1, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_BLACK_BREATH", e_name: "BLACK_BREATH", label: Some("B.Breath"), page: 3, column: 0, row: 2, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_RECHARGE", e_name: "RECHARGE", label: Some("Recharge"), page: 3, column: 0, row: 3, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_FLY", e_name: "FLY", label: Some("Fly"), page: 3, column: 0, row: 4, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_DG_CURSE", e_name: "DG_CURSE", label: Some("Mrg.Curse"), page: 3, column: 0, row: 5, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_COULD2H", e_name: "COULD2H", label: None, page: -1, column: -1, row: -1, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_MUST2H", e_name: "MUST2H", label: None, page: -1, column: -1, row: -1, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_LEVELS", e_name: "LEVELS", label: Some("Sentient"), page: 3, column: 0, row: 8, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_CLONE", e_name: "CLONE", label: Some("Clone"), page: 3, column: 0, row: 9, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_SPECIAL_GENE", e_name: "SPECIAL_GENE", label: None, page: 3, column: 0, row: 10, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_CLIMB", e_name: "CLIMB", label: Some("Climb"), page: 3, column: 0, row: 11, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_FAST_CAST", e_name: "FAST_CAST", label: None, page: -1, column: -1, row: -1, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_CAPACITY", e_name: "CAPACITY", label: None, page: -1, column: -1, row: -1, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_CHARGING", e_name: "CHARGING", label: None, page: -1, column: -1, row: -1, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_CHEAPNESS", e_name: "CHEAPNESS", label: None, page: -1, column: -1, row: -1, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_FOUNTAIN", e_name: "FOUNTAIN", label: None, page: -1, column: -1, row: -1, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_ANTIMAGIC_50", e_name: "ANTIMAGIC_50", label: None, page: -1, column: -1, row: -1, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_EASY_USE", e_name: "EASY_USE", label: None, page: -1, column: -1, row: -1, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_IM_NETHER", e_name: "IM_NETHER", label: Some("Imm Neth"), page: 1, column: 1, row: 12, kind: '*', priority: 1, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_RECHARGED", e_name: "RECHARGED", label: None, page: -1, column: -1, row: -1, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_ULTIMATE", e_name: "ULTIMATE", label: None, page: -1, column: -1, row: -1, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_LITE2", e_name: "LITE2", label: Some("Lite"), page: 2, column: 0, row: 13, kind: '2', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_LITE3", e_name: "LITE3", label: Some("Lite"), page: 2, column: 0, row: 13, kind: '3', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_FUEL_LITE", e_name: "FUEL_LITE", label: None, page: -1, column: -1, row: -1, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_CURSE_NO_DROP", e_name: "CURSE_NO_DROP", label: None, page: -1, column: -1, row: -1, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_NO_RECHARGE", e_name: "NO_RECHARGE", label: None, page: -1, column: -1, row: -1, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_TEMPORARY", e_name: "TEMPORARY", label: None, page: -1, column: -1, row: -1, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_DRAIN_MANA", e_name: "DRAIN_MANA", label: None, page: -1, column: -1, row: -1, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_DRAIN_HP", e_name: "DRAIN_HP", label: None, page: -1, column: -1, row: -1, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_KILL_DEMON", e_name: "KILL_DEMON", label: None, page: -1, column: -1, row: -1, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_KILL_UNDEAD", e_name: "KILL_UNDEAD", label: None, page: -1, column: -1, row: -1, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_CRIT", e_name: "CRIT", label: None, page: -1, column: -1, row: -1, kind: 'b', priority: 0, is_pval: true, is_esp: false },
    ObjectFlagMeta { name: "TR_ATTR_MULTI", e_name: "ATTR_MULTI", label: None, page: -1, column: -1, row: -1, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_WOUNDING", e_name: "WOUNDING", label: None, page: -1, column: -1, row: -1, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_FULL_NAME", e_name: "FULL_NAME", label: None, page: -1, column: -1, row: -1, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_LUCK", e_name: "LUCK", label: None, page: -1, column: -1, row: -1, kind: 'b', priority: 0, is_pval: true, is_esp: false },
    ObjectFlagMeta { name: "TR_IMMOVABLE", e_name: "IMMOVABLE", label: None, page: -1, column: -1, row: -1, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_SPELL_CONTAIN", e_name: "SPELL_CONTAIN", label: None, page: -1, column: -1, row: -1, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_RES_MORGUL", e_name: "RES_MORGUL", label: None, page: -1, column: -1, row: -1, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_ACTIVATE_NO_WIELD", e_name: "ACTIVATE_NO_WIELD", label: None, page: -1, column: -1, row: -1, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_MAGIC_BREATH", e_name: "MAGIC_BREATH", label: None, page: -1, column: -1, row: -1, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_WATER_BREATH", e_name: "WATER_BREATH", label: None, page: -1, column: -1, row: -1, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_WIELD_CAST", e_name: "WIELD_CAST", label: None, page: -1, column: -1, row: -1, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_RANDOM_RESIST", e_name: "RANDOM_RESIST", label: None, page: -1, column: -1, row: -1, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_RANDOM_POWER", e_name: "RANDOM_POWER", label: None, page: -1, column: -1, row: -1, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "TR_RANDOM_RES_OR_POWER", e_name: "RANDOM_RES_OR_POWER", label: None, page: -1, column: -1, row: -1, kind: 'b', priority: 0, is_pval: false, is_esp: false },
    ObjectFlagMeta { name: "ESP_ORC", e_name: "ESP_ORC", label: Some("Orc.ESP"), page: 3, column: 1, row: 0, kind: 'b', priority: 0, is_pval: false, is_esp: true },
    ObjectFlagMeta { name: "ESP_TROLL", e_name: "ESP_TROLL", label: Some("Troll.ESP"), page: 3, column: 1, row: 1, kind: 'b', priority: 0, is_pval: false, is_esp: true },
    ObjectFlagMeta { name: "ESP_DRAGON", e_name: "ESP_DRAGON", label: Some("Dragon.ESP"), page: 3, column: 1, row: 2, kind: 'b', priority: 0, is_pval: false, is_esp: true },
    ObjectFlagMeta { name: "ESP_GIANT", e_name: "ESP_GIANT", label: Some("Giant.ESP"), page: 3, column: 1, row: 3, kind: 'b', priority: 0, is_pval: false, is_esp: true },
    ObjectFlagMeta { name: "ESP_DEMON", e_name: "ESP_DEMON", label: Some("Demon.ESP"), page: 3, column: 1, row: 4, kind: 'b', priority: 0, is_pval: false, is_esp: true },
    ObjectFlagMeta { name: "ESP_UNDEAD", e_name: "ESP_UNDEAD", label: Some("Undead.ESP"), page: 3, column: 1, row: 5, kind: 'b', priority: 0, is_pval: false, is_esp: true },
    ObjectFlagMeta { name: "ESP_EVIL", e_name: "ESP_EVIL", label: Some("Evil.ESP"), page: 3, column: 1, row: 6, kind: 'b', priority: 0, is_pval: false, is_esp: true },
    ObjectFlagMeta { name: "ESP_ANIMAL", e_name: "ESP_ANIMAL", label: Some("Animal.ESP"), page: 3, column: 1, row: 7, kind: 'b', priority: 0, is_pval: false, is_esp: true },
    ObjectFlagMeta { name: "ESP_THUNDERLORD", e_name: "ESP_THUNDERLORD", label: Some("TLord.ESP"), page: 3, column: 1, row: 8, kind: 'b', priority: 0, is_pval: false, is_esp: true },
    ObjectFlagMeta { name: "ESP_GOOD", e_name: "ESP_GOOD", label: Some("Good.ESP"), page: 3, column: 1, row: 9, kind: 'b', priority: 0, is_pval: false, is_esp: true },
    ObjectFlagMeta { name: "ESP_NONLIVING", e_name: "ESP_NONLIVING", label: Some("Nlive.ESP"), page: 3, column: 1, row: 10, kind: 'b', priority: 0, is_pval: false, is_esp: true },
    ObjectFlagMeta { name: "ESP_UNIQUE", e_name: "ESP_UNIQUE", label: Some("Unique.ESP"), page: 3, column: 1, row: 11, kind: 'b', priority: 0, is_pval: false, is_esp: true },
    ObjectFlagMeta { name: "ESP_SPIDER", e_name: "ESP_SPIDER", label: Some("Spider ESP"), page: 3, column: 1, row: 12, kind: 'b', priority: 0, is_pval: false, is_esp: true },
    ObjectFlagMeta { name: "ESP_ALL", e_name: "ESP_ALL", label: Some("Full ESP"), page: 3, column: 1, row: 15, kind: 'b', priority: 0, is_pval: false, is_esp: true },
];

/// `object_flag_metas_by_pcr` (files.cc:1712): all metas mapped to one
/// page/column/row, in list order.  Metas without a name are not mapped
/// (the original skips `!c_name`).
#[allow(dead_code)]
pub fn object_flag_metas_by_pcr(
    metas: &[ObjectFlagMeta],
    page: i32,
    column: i32,
    row: i32,
) -> Vec<usize> {
    metas
        .iter()
        .enumerate()
        .filter(|(_, m)| {
            m.label.is_some() && m.page == page && m.column == column && m.row == row
        })
        .map(|(i, _)| i)
        .collect()
}

/// `get_lowest_priority_object_flag_meta` (files.cc:1936): the entry with
/// the smallest `c_priority`, first one wins ties.
#[allow(dead_code)]
pub fn get_lowest_priority_object_flag_meta(metas: &[ObjectFlagMeta]) -> Option<usize> {
    let mut found: Option<usize> = None;
    for (i, m) in metas.iter().enumerate() {
        match found {
            None => found = Some(i),
            Some(f) if metas[f].priority > m.priority => found = Some(i),
            _ => {}
        }
    }
    found
}

// --- Player location (files.cc:2270) --------------------------------------

/// `describe_player_location` (files.cc:2270): "on level N of <dungeon>",
/// "in the town of X", "near X", or the wilderness text with an optional
/// direction to the nearest known landmark.  `player_pos` is the ECS
/// position used while `wild_mode` is on (the original's `px`/`py`).
#[allow(dead_code)]
pub fn describe_player_location(
    gd: &GameData,
    wilderness: &Wilderness,
    plot: &PlotQuest,
    ps: &PlayerState,
    player_pos: (i32, i32),
) -> String {
    let pwx = if ps.wild_mode { player_pos.0 } else { ps.wild_x };
    let pwy = if ps.wild_mode { player_pos.1 } else { ps.wild_y };
    let feat = wilderness.wf_at(gd, pwx, pwy, plot);

    let mut desc = String::new();
    if ps.dungeon != crate::base_defs::DUNGEON_WILDERNESS as u32 && ps.depth > 0 {
        desc += &format!("on level {} of {}", ps.depth, gd.dungeon(ps.dungeon).name);
    } else if feat.terrain_idx == crate::base_defs::TERRAIN_TOWN as u32 {
        desc += &format!("in the town of {}", feat.name);
    } else if feat.entrance != 0 {
        desc += &format!("near {}", feat.name);
    } else {
        // init1.cc:6479 records the first world cell of every entrance
        // terrain; use the unpatched map for that, like the original.
        let mut first: HashMap<u32, (i32, i32)> = HashMap::new();
        for y in 0..gd.world.h {
            for x in 0..gd.world.w {
                let wf = gd.wf(gd.world.feat(x, y, &[false; 6]));
                if wf.entrance != 0 {
                    first.entry(wf.id).or_insert((x, y));
                }
            }
        }

        let mut landmark: Option<(u32, i32, i32)> = None;
        let mut l_dist = -1;
        for wf in &gd.wf {
            if wf.entrance == 0 {
                continue;
            }
            let (wx, wy) = first.get(&wf.id).copied().unwrap_or((0, 0));
            if !wilderness.is_known(wx, wy) {
                continue;
            }
            let dist = crate::map::pref_distance(wy, wx, pwy, pwx);
            if l_dist < 0 || dist < l_dist {
                landmark = Some((wf.id, wx, wy));
                l_dist = dist;
            }
        }

        match landmark {
            None => desc += &format!("in {}", feat.text),
            Some((id, lwx, lwy)) => {
                if pwx == lwx && pwy == lwy {
                    desc += &format!("near {}", gd.wf(id).name);
                } else {
                    let mut dx = pwx - lwx;
                    let mut dy = pwy - lwy;
                    let mut ns = if dy > 0 { "south" } else { "north" };
                    let mut ew = if dx > 0 { "east" } else { "west" };
                    dx = dx.abs();
                    dy = dy.abs();
                    if dy * 81 < dx * 31 {
                        ns = "";
                    }
                    if dx * 81 < dy * 31 {
                        ew = "";
                    }
                    desc += &format!("in {} {}{} of {}", feat.text, ns, ew, gd.wf(id).name);
                }
            }
        }
    }

    desc.trim_end().to_string()
}

// --- Retire / quit / save (files.cc:3691-5070) ----------------------------

/// `process_player_name` (files.cc:3691): names are at most 15 bytes, may
/// not contain control characters, keep alphanumerics and map `@. _` to
/// `_`; an empty result becomes "PLAYER".  The original quits on the two
/// fatal cases; the port returns an error instead.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerNameError {
    TooLong,
    ControlChar,
}

#[allow(dead_code)]
pub fn process_player_name(name: &str) -> Result<String, PlayerNameError> {
    if name.len() > 15 {
        return Err(PlayerNameError::TooLong);
    }
    if name.chars().any(|c| c.is_control()) {
        return Err(PlayerNameError::ControlChar);
    }
    let mut buf = String::new();
    for c in name.chars() {
        if c.is_alphanumeric() {
            buf.push(c);
        } else if matches!(c, '@' | '.' | ' ' | '_') {
            buf.push('_');
        }
    }
    if buf.is_empty() {
        buf.push_str("PLAYER");
    }
    Ok(buf)
}

/// `set_player_base` (files.cc:3738): `game->player_base` is the
/// sanitised player name.
#[allow(dead_code)]
pub fn set_player_base(name: &str) -> Result<String, PlayerNameError> {
    process_player_name(name)
}

/// `get_name` (files.cc:3754) is a prompt loop (HUD); what it commits is
/// `set_player_base`, which is ported above.

/// `kingly` (files.cc:4905): the retirement state changes (the crown
/// drawing is HUD).  `max_exp` is not tracked separately in the port, so
/// the experience floor is the level requirement.
#[allow(dead_code)]
pub fn kingly(ps: &mut PlayerState) {
    ps.depth = 0;
    ps.died_from = "Ripe Old Age".to_string();
    ps.level = ps.max_plv;
    ps.exp = ps.exp.max(ps.exp_needed());
    ps.gold += 10_000_000;
}

/// `remove_cave_view` (files.cc:3848): clear (or restore) the CAVE_VIEW
/// marker of every cell in the view list.  The Bevy port recomputes
/// `Map::visible` from the FOV, so this keeps the original's set/clear
/// semantics for the cells involved.
#[allow(dead_code)]
pub fn remove_cave_view(visible: &mut [bool], view_cells: &[usize], remove: bool) {
    for &i in view_cells {
        visible[i] = !remove;
    }
}

/// `wipe_saved` (files.cc:4957): the (dungeon, depth) pairs the original
/// scans for persistent level savefiles.  The port keeps a single
/// savegame (deleted by `save::delete`), so only the scan remains.
#[allow(dead_code)]
pub fn wipe_saved_levels(dungeons: &[DungeonDef]) -> Vec<(u32, u32)> {
    let mut out = Vec::new();
    for d in dungeons {
        for l in d.mindepth..=d.maxdepth {
            out.push((d.id, l));
        }
    }
    out
}

/// The maximum number of high scores (`MAX_HISCORES`).
pub const MAX_HISCORES: usize = 100;

/// `highscore_where` (hiscore.cc:26) / `predict_score` (files.cc:4747):
/// the 0-based slot a new score would take in a descending list.  A short
/// list returns its length; a full list caps at the last slot.
#[allow(dead_code)]
pub fn score_placement(existing: &[u64], score: u64) -> usize {
    let n = existing.len().min(MAX_HISCORES);
    for i in 0..n {
        if existing[i] < score {
            return i;
        }
    }
    if n < MAX_HISCORES {
        n
    } else {
        MAX_HISCORES - 1
    }
}

/// `do_cmd_help` (files.cc:3676) opens `help.hlp`; the port ships the
/// same help tree under `bevy/assets/data/help` and shows it in a modal
/// text page (`modal::clean_help_line`).
#[allow(dead_code)]
pub fn help_file() -> &'static str {
    include_str!("../assets/data/help/help.hlp")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn likert_matches_the_original_bands() {
        assert_eq!(likert(-1, 10), "Very Bad");
        assert_eq!(likert(10, 10), "Bad");
        assert_eq!(likert(29, 10), "Poor");
        assert_eq!(likert(40, 10), "Fair");
        assert_eq!(likert(50, 10), "Good");
        assert_eq!(likert(60, 10), "Very Good");
        assert_eq!(likert(80, 10), "Excellent");
        assert_eq!(likert(120, 10), "Superb");
        assert_eq!(likert(160, 10), "Heroic");
        assert_eq!(likert(200, 10), "Legendary[7]");
        assert_eq!(likert(5, 0), likert(5, 1));
    }

    #[test]
    fn number_to_digit_caps_at_star() {
        assert_eq!(number_to_digit(0), '0');
        assert_eq!(number_to_digit(-7), '7');
        assert_eq!(number_to_digit(9), '9');
        assert_eq!(number_to_digit(10), '*');
        assert_eq!(number_to_digit(-100), '*');
    }

    #[test]
    fn flag_cells_append_by_type() {
        let n = |p| ObjectFlagCell { kind: 'n', pval: p };
        let b = ObjectFlagCell {
            kind: 'b',
            pval: 0,
        };
        let plus = ObjectFlagCell {
            kind: '+',
            pval: 0,
        };
        let star = ObjectFlagCell {
            kind: '*',
            pval: 0,
        };
        assert_eq!(object_flag_cell_append(OBJECT_FLAG_CELL_EMPTY, n(3)), n(3));
        assert_eq!(object_flag_cell_append(n(3), OBJECT_FLAG_CELL_EMPTY), n(3));
        assert_eq!(object_flag_cell_append(n(2), n(5)), n(7));
        assert_eq!(object_flag_cell_append(plus, star), star);
        assert_eq!(object_flag_cell_append(star, plus), star);
        assert_eq!(object_flag_cell_append(plus, plus), plus);
        assert_eq!(object_flag_cell_append(b, b), b);
        assert!(object_flag_types_are_compatible('\0', 'n'));
        assert!(object_flag_types_are_compatible('+', '*'));
        assert!(!object_flag_types_are_compatible('n', 'b'));
    }

    #[test]
    fn flag_cell_chars_follow_the_original() {
        assert_eq!(object_flag_cell_to_char(OBJECT_FLAG_CELL_EMPTY), ('.', 0));
        assert_eq!(
            object_flag_cell_to_char(ObjectFlagCell {
                kind: 'n',
                pval: 0
            }),
            ('+', 1)
        );
        assert_eq!(
            object_flag_cell_to_char(ObjectFlagCell {
                kind: 'n',
                pval: -3
            }),
            ('3', -1)
        );
        assert_eq!(
            object_flag_cell_to_char(ObjectFlagCell {
                kind: 'b',
                pval: 0
            }),
            ('+', 1)
        );
        assert_eq!(
            object_flag_cell_to_char(ObjectFlagCell {
                kind: '*',
                pval: 0
            }),
            ('*', 1)
        );
    }

    #[test]
    fn get_line_reads_nth_line() {
        let dir = std::env::temp_dir().join(format!("tome-files-getline-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("f.txt");
        std::fs::write(&p, "first\nsecond\nthird\n").unwrap();
        assert_eq!(get_line(&p, 1).as_deref(), Some("second"));
        assert_eq!(get_line(&p, 9), None);
        assert_eq!(get_line(&dir.join("missing"), 0), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn timed_autosave_due_matches_the_original_formula() {
        assert!(!timed_autosave_due(100, 5, false));
        assert!(!timed_autosave_due(100, 0, true));
        assert!(timed_autosave_due(50, 5, true));
        assert!(!timed_autosave_due(51, 5, true));
    }

    #[test]
    fn name_file_helpers_follow_the_original() {
        assert_eq!(name_file_note("Frodo"), "Frodo.nte");
        assert_eq!(name_file_pref("user"), "user.prf");
        // The port's single save slot.
        assert_eq!(name_file_save(), std::path::PathBuf::from(crate::save::path()));
        let named = name_file_save_named("Hero");
        assert!(named.ends_with("Hero"));
        let lev = name_file_dungeon_save("lev");
        assert_eq!(lev.extension().and_then(|e| e.to_str()), Some("lev"));
        assert_eq!(lev.parent(), name_file_save().parent());
    }

    #[test]
    fn tokenize_splits_and_repairs_like_the_original() {
        // Both delimiters split; the last token keeps delimiters.
        assert_eq!(tokenize("a:b/c", 3, ':', '/'), vec!["a", "b", "c"]);
        assert_eq!(tokenize("a:b:c:d", 3, ':', '/'), vec!["a", "b", "c:d"]);
        assert_eq!(tokenize("a:b", 5, ':', '/'), vec!["a", "b"]);
        // Empty buffer yields one empty token, exactly like the original.
        assert_eq!(tokenize("", 3, ':', '/'), vec![""]);
        // Backslash escapes the delimiter.
        assert_eq!(tokenize("a\\:b", 2, ':', '/'), vec!["a\\:b"]);
        // The quote formalism keeps the quoted piece together and
        // repairs a missing closing quote in place (`X` becomes `'`).
        assert_eq!(tokenize("'a':b", 2, ':', '/'), vec!["'a'", "b"]);
        assert_eq!(tokenize("'aX:b", 2, ':', '/'), vec!["'a'", "b"]);
    }

    #[test]
    fn pref_expr_evaluates_like_the_original() {
        fn eval(s: &str) -> String {
            let chars: Vec<char> = s.chars().collect();
            let mut pos = 0;
            let mut f = ' ';
            let mut skill = |_: &str| -> Option<i32> { None };
            process_pref_file_expr(&chars, &mut pos, &mut f, &mut skill)
        }
        assert_eq!(eval("hello"), "hello");
        assert_eq!(eval("$SYS"), ANGBAND_SYS);
        assert_eq!(eval("$OTHER"), "?o?o?");
        assert_eq!(eval("[IOR 0 0 1]"), "1");
        assert_eq!(eval("[IOR 0 0]"), "0");
        assert_eq!(eval("[AND 1 0]"), "0");
        assert_eq!(eval("[NOT 0]"), "1");
        assert_eq!(eval("[EQU 1 1 1]"), "1");
        assert_eq!(eval("[EQU 1 2]"), "0");
        assert_eq!(eval("[EQU $SYS bevy]"), "1");
        // Unknown functions leave the default value.
        assert_eq!(eval("[BOGUS 1]"), "?o?o?");
        // SKILL resolves through the supplied lookup.
        let chars: Vec<char> = "[SKILL Stealth]".chars().collect();
        let (mut pos, mut f) = (0, ' ');
        let mut skill = |name: &str| -> Option<i32> {
            if name == "Stealth" {
                Some(24)
            } else {
                None
            }
        };
        assert_eq!(
            process_pref_file_expr(&chars, &mut pos, &mut f, &mut skill),
            "24"
        );
    }

    #[test]
    fn pref_file_processes_includes_and_conditionals() {
        let dir = std::env::temp_dir().join(format!("tome-pref-{}", std::process::id()));
        let user = dir.join("user");
        let pref = dir.join("pref");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&user).unwrap();
        std::fs::create_dir_all(&pref).unwrap();
        std::fs::write(
            user.join("outer.prf"),
            "V:1\n?:0\nX:skipped\n%:inner.prf\n?:1\nV:2\n%:inner.prf\n",
        )
        .unwrap();
        std::fs::write(pref.join("inner.prf"), "V:3\n").unwrap();

        let mut seen: Vec<String> = Vec::new();
        let mut aux = |line: &str| {
            seen.push(line.to_string());
            0
        };
        let mut skill = |_: &str| None;
        let (err, msgs) = process_pref_file("outer.prf", &user, &pref, &mut aux, &mut skill);
        assert_eq!(err, 0);
        assert!(msgs.is_empty());
        assert_eq!(seen, vec!["V:1", "V:2", "V:3"]);

        // An error stops the file and produces the original messages.
        let mut aux = |line: &str| if line == "V:2" { 7 } else { 0 };
        let (err, msgs) = process_pref_file("outer.prf", &user, &pref, &mut aux, &mut skill);
        assert_eq!(err, 7);
        assert!(msgs[0].contains("Error 7 in line"));
        assert!(msgs[0].contains("outer.prf"));

        // Missing files report -1.
        let (err, _) = process_pref_file("nope.prf", &user, &pref, &mut aux, &mut skill);
        assert_eq!(err, -1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn character_dump_helpers_match_the_original() {
        // Check-row only looks at columns 12+.
        assert!(!file_character_print_grid_check_row("           plain"));
        assert!(file_character_print_grid_check_row("             +  "));
        assert!(file_character_print_grid_check_row("            7"));
        assert!(!file_character_print_grid_check_row("+ short"));

        // Grid layout: the left half's y=3 row is always printed when
        // non-blank, gaps force rows, the right half never has y=3.
        let blank = " ".repeat(40);
        let mut left = vec![blank.clone(); 23];
        let mut right = vec![blank.clone(); 23];
        left[3] = "top".to_string();
        left[5] = format!("{:12}+", "");
        left[6] = blank.clone();
        right[3] = "right-top".to_string();
        right[5] = format!("{:12}9", "");
        let lines = file_character_print_grid_lines(&left, &right, false, true);
        assert_eq!(
            lines,
            vec![
                "        top",
                "                    +",
                "                    9"
            ]
        );
        let lines = file_character_print_grid_lines(&left, &right, true, false);
        assert_eq!(lines.len(), 2, "{lines:?}");

        // Item / store text and the dump header.
        assert_eq!(
            file_character_print_item_text('a', "a Long Sword"),
            "a) a Long Sword\n"
        );
        assert_eq!(file_character_print_store_text("Home", "Bree", &[]), "");
        let text = file_character_print_store_text(
            "Home",
            "Bree",
            &["first".to_string(), "second".to_string()],
        );
        assert_eq!(
            text,
            "  [Home Inventory - Bree]\n\na) first\nb) second\n\n\n"
        );
        assert_eq!(
            character_sheet_header("1.0.0"),
            "  [1.0.0 Character Sheet]\n\n"
        );
    }

    #[test]
    fn object_flag_meta_index_and_priority() {
        let metas = vec![
            ObjectFlagMeta {
                page: 1,
                column: 0,
                row: 2,
                priority: 5,
                label: Some("A"),
                ..Default::default()
            },
            ObjectFlagMeta {
                page: 1,
                column: 0,
                row: 2,
                priority: 2,
                label: Some("B"),
                ..Default::default()
            },
            ObjectFlagMeta {
                page: 1,
                column: 1,
                row: 2,
                priority: 1,
                label: Some("C"),
                ..Default::default()
            },
            ObjectFlagMeta {
                page: 1,
                column: 0,
                row: 2,
                priority: 0,
                label: None,
                ..Default::default()
            },
        ];
        assert_eq!(object_flag_metas_by_pcr(&metas, 1, 0, 2), vec![0, 1]);
        assert_eq!(object_flag_metas_by_pcr(&metas, 1, 1, 2), vec![2]);
        assert_eq!(object_flag_metas_by_pcr(&metas, 9, 9, 9), Vec::<usize>::new());
        // Named entries are the only ones mapped; the unnamed one is not.
        assert_eq!(get_lowest_priority_object_flag_meta(&metas[0..2]), Some(1));
        assert_eq!(get_lowest_priority_object_flag_meta(&[]), None);
    }

    #[test]
    fn monoid_mconcat_folds_with_append_and_empty() {
        // The generic monoid (monoid.hpp:16/35) instantiated for
        // object_flag_cell: mconcat = fold append empty.
        let cells = [
            ObjectFlagCell { kind: 'n', pval: 2 },
            ObjectFlagCell { kind: 'n', pval: 5 },
            ObjectFlagCell { kind: 'n', pval: -1 },
        ];
        let got = mconcat(&cells, OBJECT_FLAG_CELL_EMPTY, object_flag_cell_append);
        assert_eq!(got, ObjectFlagCell { kind: 'n', pval: 6 });
        // Empty is the identity on both sides.
        assert_eq!(
            object_flag_cell_append(OBJECT_FLAG_CELL_EMPTY, cells[0]),
            cells[0]
        );
        assert_eq!(
            object_flag_cell_append(cells[0], OBJECT_FLAG_CELL_EMPTY),
            cells[0]
        );
        // The fold is left-to-right and associative for this operation.
        let left = mconcat(
            &[
                object_flag_cell_append(cells[0], cells[1]),
                cells[2],
            ],
            OBJECT_FLAG_CELL_EMPTY,
            object_flag_cell_append,
        );
        assert_eq!(left, got);
        // The ternary supercedes rule survives the fold: + then * = *.
        let ternary = [
            ObjectFlagCell { kind: '+', pval: 0 },
            ObjectFlagCell { kind: '*', pval: 0 },
        ];
        assert_eq!(
            mconcat(&ternary, OBJECT_FLAG_CELL_EMPTY, object_flag_cell_append),
            ObjectFlagCell { kind: '*', pval: 0 }
        );
        assert_eq!(mconcat::<i32>(&[], 0, |a, b| a + b), 0);
    }

    #[test]
    fn object_flags_meta_and_esp_match_the_original_list() {
        // object_flag_meta.cc:5 expands object_flag_list.hpp; the list has
        // 156 flags (TR lines), the first is STR and the last ESP_ALL.
        let metas = object_flags_meta();
        assert_eq!(metas.len(), 156);
        let first = metas[0];
        assert_eq!(first.name, "TR_STR");
        assert_eq!(first.e_name, "STR");
        assert_eq!(first.label, Some("Add Str"));
        assert_eq!((first.page, first.column, first.row), (0, 0, 0));
        assert_eq!(first.kind, 'n');
        assert!(first.is_pval && !first.is_esp);
        let last = metas[155];
        assert_eq!(last.name, "ESP_ALL");
        assert_eq!(last.label, Some("Full ESP"));
        assert!(last.is_esp);
        // Unnamed flags keep the -1 slot (files.cc skips !c_name).
        let decay = metas.iter().find(|m| m.name == "TR_DECAY").unwrap();
        assert_eq!(decay.label, None);
        assert_eq!((decay.page, decay.column, decay.row), (-1, -1, -1));
        // TERNARY/FIXED expand to their character-sheet chars.
        assert_eq!(
            metas.iter().find(|m| m.name == "TR_RES_ACID").unwrap().kind,
            '+'
        );
        assert_eq!(
            metas.iter().find(|m| m.name == "TR_IM_ACID").unwrap().kind,
            '*'
        );
        assert_eq!(
            metas.iter().find(|m| m.name == "TR_LITE1").unwrap().kind,
            '1'
        );
        assert_eq!(
            metas.iter().find(|m| m.name == "TR_LITE3").unwrap().kind,
            '3'
        );
        // is_pval is set 14 times (NUMERIC stats + CRIT/LUCK/SPELL).
        assert_eq!(metas.iter().filter(|m| m.is_pval).count(), 14);
        // object_flags_esp: the 14 ESP_ flags, in list order.
        let esp = object_flags_esp();
        assert_eq!(esp.len(), 14);
        assert_eq!(esp[0], "ESP_ORC");
        assert_eq!(esp[13], "ESP_ALL");
        assert!(esp.contains(&"ESP_UNDEAD") && !esp.contains(&"STR"));
    }

    #[test]
    fn describe_player_location_follows_the_world() {
        let gd = crate::data::load_game_data();
        let mut rng = crate::rng::new_seeded_rng(7);
        let mut wilderness = Wilderness::new(&gd, &mut rng);
        for k in wilderness.known.iter_mut() {
            *k = true;
        }
        let plot = PlotQuest::default();
        let mut ps = crate::birth::make_player(&gd, "Loc".into(), 0, 0);
        ps.wild_mode = false;
        ps.depth = 0;
        ps.dungeon = 0;

        // A town cell describes the town.
        let mut town_cell = None;
        let mut entrance_cell = None;
        let mut plain_cell = None;
        'scan: for y in 0..gd.world.h {
            for x in 0..gd.world.w {
                let wf = gd.wf(gd.world.feat(x, y, &[false; 6]));
                if wf.town() > 0 && town_cell.is_none() {
                    town_cell = Some((x, y));
                }
                if wf.entrance != 0 && wf.town() == 0 && entrance_cell.is_none() {
                    entrance_cell = Some((x, y));
                }
                if wf.entrance == 0 && plain_cell.is_none() && x >= 5 && y >= 5 {
                    plain_cell = Some((x, y));
                }
                if town_cell.is_some() && entrance_cell.is_some() && plain_cell.is_some() {
                    break 'scan;
                }
            }
        }
        let (x, y) = town_cell.expect("town cell");
        ps.wild_x = x;
        ps.wild_y = y;
        let desc = describe_player_location(&gd, &wilderness, &plot, &ps, (0, 0));
        let wf = gd.wf(gd.world.feat(x, y, &[false; 6]));
        assert_eq!(desc, format!("in the town of {}", wf.name.trim_end()));

        let (x, y) = entrance_cell.expect("dungeon entrance");
        ps.wild_x = x;
        ps.wild_y = y;
        let desc = describe_player_location(&gd, &wilderness, &plot, &ps, (0, 0));
        let wf = gd.wf(gd.world.feat(x, y, &[false; 6]));
        assert_eq!(desc, format!("near {}", wf.name.trim_end()));

        // A dungeon level names the dungeon.
        ps.depth = 5;
        ps.dungeon = 1;
        let desc = describe_player_location(&gd, &wilderness, &plot, &ps, (0, 0));
        assert_eq!(desc, format!("on level 5 of {}", gd.dungeon(1).name));

        // Plain wilderness points at the nearest known landmark.
        ps.depth = 0;
        ps.dungeon = 0;
        let (x, y) = plain_cell.expect("plain cell");
        ps.wild_x = x;
        ps.wild_y = y;
        let desc = describe_player_location(&gd, &wilderness, &plot, &ps, (0, 0));
        assert!(desc.starts_with("in "), "{desc}");
        assert!(desc.contains(" of "), "{desc}");
    }

    #[test]
    fn process_player_name_sanitizes() {
        assert_eq!(process_player_name("Frodo"), Ok("Frodo".to_string()));
        assert_eq!(process_player_name("Fro do"), Ok("Fro_do".to_string()));
        assert_eq!(process_player_name("a@b.c_d"), Ok("a_b_c_d".to_string()));
        assert_eq!(process_player_name("!!!"), Ok("PLAYER".to_string()));
        assert_eq!(set_player_base("Hero"), Ok("Hero".to_string()));
        assert_eq!(
            process_player_name("1234567890123456"),
            Err(PlayerNameError::TooLong)
        );
        assert_eq!(
            process_player_name("a\nb"),
            Err(PlayerNameError::ControlChar)
        );
    }

    #[test]
    fn kingly_retires_the_winner() {
        let gd = crate::data::load_game_data();
        let mut ps = crate::birth::make_player(&gd, "King".into(), 0, 0);
        ps.max_plv = 30;
        ps.level = 10;
        ps.exp = 0;
        ps.gold = 5;
        ps.depth = 7;
        kingly(&mut ps);
        assert_eq!(ps.depth, 0);
        assert_eq!(ps.died_from, "Ripe Old Age");
        assert_eq!(ps.level, 30);
        assert_eq!(ps.gold, 10_000_005);
        assert!(ps.exp >= ps.exp_needed());
    }

    #[test]
    fn remove_cave_view_sets_and_clears_the_flags() {
        let mut visible = vec![false; 5];
        remove_cave_view(&mut visible, &[1, 3], false);
        assert_eq!(visible, vec![false, true, false, true, false]);
        remove_cave_view(&mut visible, &[3], true);
        assert_eq!(visible, vec![false, true, false, false, false]);
    }

    #[test]
    fn wipe_saved_levels_scans_every_dungeon_depth() {
        let gd = crate::data::load_game_data();
        let pairs = wipe_saved_levels(&gd.dungeons);
        let expected: usize = gd
            .dungeons
            .iter()
            .map(|d| (d.maxdepth - d.mindepth + 1) as usize)
            .sum();
        assert_eq!(pairs.len(), expected);
        let d1 = gd.dungeon(1);
        assert!(pairs.contains(&(1, d1.mindepth)));
        assert!(pairs.contains(&(1, d1.maxdepth)));
    }

    #[test]
    fn score_placement_matches_highscore_where() {
        assert_eq!(score_placement(&[100, 50, 10], 200), 0);
        assert_eq!(score_placement(&[100, 50, 10], 75), 1);
        // A short list that cannot beat anything returns its length.
        assert_eq!(score_placement(&[100, 50, 10], 5), 3);
        // A full list caps at the last slot.
        let full = vec![10u64; MAX_HISCORES];
        assert_eq!(score_placement(&full, 5), MAX_HISCORES - 1);
        assert_eq!(score_placement(&full, 20), 0);
    }

    #[test]
    fn screenshot_formatters_follow_the_original() {
        // cmovie_clean_line: map columns use the map attr/char with '\0'
        // turned into a space, other columns the terminal contents.
        let (a, c) = cmovie_clean_line(
            3,
            &|x| if x < 2 { Some((4u8, if x == 0 { '\0' } else { 'X' })) } else { None },
            &|_| (15u8, 'U'),
        );
        assert_eq!(a, vec!['r', 'r', 'U']);
        assert_eq!(c, vec![' ', 'X', 'U']);

        let rows = vec![(vec!['r', 'w'], vec!['A', 'B'])];
        assert_eq!(help_file_screenshot_text(&rows), "&&&&&rAwB\n");

        let html = html_screenshot_text(
            "shot",
            "1.2.3",
            &crate::colors::PALETTE,
            &[(vec!['w', 'w'], vec!['<', '&'])],
        );
        assert!(html.contains("<title>shot</title>"));
        assert!(html.contains("content=\"1.2.3\""));
        assert!(html.contains("&lt;&amp;"));
        assert!(html.contains("</html>"));
    }

    #[test]
    fn wielded_monster_flags_flow_into_totals() {
        use crate::data::{SLOT_SYMBIOTE, TV_HYPNOS};
        use crate::item::{Inventory, Item};
        let gd = crate::data::load_game_data();
        let hyp = gd.object_by_tval_sval(TV_HYPNOS, 1).expect("symbiote kind");
        let build = |flag: &str| {
            let def = gd
                .monsters
                .iter()
                .position(|m| m.has(flag))
                .unwrap_or_else(|| panic!("no monster with {flag}"));
            let mut it = Item::base(&gd, hyp);
            it.note = def as u32;
            let mut inv = Inventory::default();
            inv.equip[SLOT_SYMBIOTE] = Some(it);
            inv
        };
        assert!(build("INVISIBLE").totals(&gd).invis);
        assert!(build("REFLECTING").totals(&gd).reflect);
        assert!(build("CAN_FLY").totals(&gd).feather);
        assert!(build("AQUATIC").totals(&gd).water_breath);
    }

    #[test]
    fn suicide_clears_last_chance_deaths() {
        let gd = crate::data::load_game_data();
        let mut ps = crate::birth::make_player(&gd, "Quitter".into(), 0, 0);
        ps.allow_one_death = 1;
        ps.ring_worn = true;
        ps.undead_form = Some(3);
        let mut log = crate::game::MessageLog::default();
        crate::game::do_cmd_suicide(&mut ps, &mut log);
        assert_eq!(ps.hp, 0);
        assert_eq!(ps.allow_one_death, 0);
        assert!(!ps.ring_worn);
        assert!(ps.undead_form.is_none());
        assert!(log.lines.iter().any(|l| l.contains("suicide")));
    }

    #[test]
    fn race_scores_filter_and_rank() {
        use crate::scores::{HighScores, ScoreEntry};
        let entry = |name: &str, race: &str, level: u32| ScoreEntry {
            name: name.to_string(),
            race: race.to_string(),
            class: "Warrior".to_string(),
            level,
            max_depth: 10,
            exp: 0,
            turns: 0,
            gold: 0,
            score: level as u64 * 100,
            how: "Death".to_string(),
            won: false,
        };
        let hs = HighScores {
            entries: vec![
                entry("A", "Hobbit", 50),
                entry("B", "Hobbit", 20),
                entry("C", "Ent", 30),
            ],
        };
        let hobbits: Vec<&str> = hs.by_race("Hobbit").map(|e| e.name.as_str()).collect();
        assert_eq!(hobbits, vec!["A", "B"]);
        assert_eq!(hs.kings().count(), 1);
        assert_eq!(hs.by_class("Warrior").count(), 3);
    }

    #[test]
    fn help_asset_is_the_ported_help_file() {
        assert!(help_file().contains("Online Help System"));
        assert!(help_file().contains("help.hlp"));
    }

    #[test]
    fn get_line_skips_the_leading_comment_like_the_original() {
        let dir = std::env::temp_dir().join(format!("tome-files-comment-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("f.txt");
        std::fs::write(&p, "comment\nfirst\nsecond\n").unwrap();
        // C `get_line(.., 0)` skips the first (comment) line and returns
        // the second, i.e. 0-based index 1 in the port.
        assert_eq!(get_line(&p, 1).as_deref(), Some("first"));
        assert_eq!(get_line(&p, 2).as_deref(), Some("second"));
        assert_eq!(get_line(&p, 3), None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
