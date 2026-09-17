//! Port of the non-terminal parts of `src/util.cc`.
//!
//! The original file is dominated by terminal I/O (`Term_*`, `inkey`,
//! `prt`, `screen_save`).  The Bevy port renders through ECS entities and
//! routes keys through `input.rs`/`modal.rs`, so this module ports the
//! *logic* those helpers contain: screen-row writes, the input parsers
//! (`get_number`, `askfor_aux`, `get_check`, `get_com`, `get_quantity`,
//! `request_command`), the path/file helpers, the ASCII <-> printable
//! converters and the macro tables.  Interactive entry points take a key
//! callback so the exact original key sequences can be tested.

use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufRead, Read, Seek, SeekFrom, Write};

/// `ESCAPE` (h-basic.hpp:134).
pub const ESCAPE: char = '\u{1b}';
/// `REPEAT_MAX` (util.cc:3191).
pub const REPEAT_MAX: usize = 20;
/// `MAX_IGNORE_KEYMAPS` (util.hpp:9).
pub const MAX_IGNORE_KEYMAPS: usize = 12;
/// The commands that auto-repeat under `always_repeat` (util.cc:3058).
pub const AUTO_REPEAT_CMDS: &str = "TBDoc+";

/// `D2I(X)` (h-basic.hpp:131).
pub fn d2i(c: char) -> i32 {
    c as i32 - '0' as i32
}

/// `A2I(X)` (h-basic.hpp:128).
pub fn a2i(c: char) -> i32 {
    c as i32 - 'a' as i32
}

/// `KTRL(X)` (h-basic.hpp:133).
pub fn ktrl(c: char) -> char {
    ((c as u32) & 0x1f) as u8 as char
}

// ---------------------------------------------------------------------------
// get_number / get_count
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplayOption {
    Immediate,
    Delay,
}

/// `get_number` (util.cc:55): read a number, digit by digit, starting
/// from `def` and capped at `max`.
pub struct NumberInput {
    pub res: u32,
    pub max: u32,
    no_keys: bool,
}

impl NumberInput {
    pub fn new(def: u32, max: u32) -> Self {
        NumberInput {
            res: def,
            max,
            no_keys: true,
        }
    }

    /// The `display_option_t::IMMEDIATE` rendering of the initial value.
    pub fn initial_display(&self, option: DisplayOption) -> Option<u32> {
        match option {
            DisplayOption::Immediate => Some(self.res),
            DisplayOption::Delay => None,
        }
    }

    /// Feed one keypress.  `Some((number, key))` mirrors the original's
    /// return, `None` while the editor consumes the key.
    pub fn key(&mut self, key: char) -> Option<(u32, char)> {
        if key == '\u{7f}' || key == '\u{8}' {
            // Simple editing (delete or backspace).
            self.no_keys = false;
            self.res /= 10;
            None
        } else if key.is_ascii_digit() {
            // Override the default on the first real digit.
            if self.no_keys {
                self.no_keys = false;
                self.res = 0;
            }
            let d = d2i(key) as u32;
            // Don't overflow: (u32b)(0 - 1) - D2I(key)) / 10 < res.
            if (u32::MAX - d) / 10 < self.res {
                self.res = if self.max == u32::MAX { u32::MAX } else { self.max };
            } else if self.res * 10 + d > self.max {
                // Stop count at maximum.
                self.res = self.max;
            } else {
                self.res = self.res * 10 + d;
            }
            None
        } else if key == ESCAPE {
            // Escape cancels.
            Some((0, key))
        } else {
            Some((self.res, key))
        }
    }
}

/// `get_count` (util.cc:3269): "How many?" over a `get_number`.
pub fn get_count(number: i32, max: i32, input: &mut dyn FnMut() -> char) -> i32 {
    let mut n = NumberInput::new(number as u32, max as u32);
    loop {
        if let Some((res, _cmd)) = n.key(input()) {
            return res as i32;
        }
    }
}

// ---------------------------------------------------------------------------
// user name, paths and file helpers
// ---------------------------------------------------------------------------

/// `user_name` (util.cc:157): the login name, "PLAYER" when unknown.
pub fn user_name_from(env_user: Option<&str>) -> String {
    match env_user {
        Some(u) if !u.is_empty() => u.to_string(),
        _ => "PLAYER".to_string(),
    }
}

pub fn user_name() -> String {
    user_name_from(std::env::var("USER").ok().as_deref())
}

/// The path separator (`PATH_SEP`, "/" on the original's normal systems).
pub const PATH_SEP: &str = "/";

/// `path_parse` (util.cc:214, the SET_UID branch): a leading `~/` is
/// replaced by the current user's home directory.  The user name between
/// `~` and the next separator is ignored (the original only ever looks up
/// the current uid).  `None` mirrors a NULL `file` pointer.
pub fn path_parse(file: Option<&str>, home: &str) -> Option<String> {
    let file = file?;
    if !file.starts_with('~') {
        return Some(file.to_string());
    }
    let rest = &file[1..];
    let mut out = home.to_string();
    if let Some(slash) = rest.find(PATH_SEP) {
        out.push_str(&rest[slash..]);
    }
    Some(out)
}

/// `path_build` (util.cc:300): append `file` to `path`, unless the file
/// is special (`~` or absolute) or no path was given.
pub fn path_build(path: &str, file: &str) -> String {
    if file.starts_with('~') || file.starts_with(PATH_SEP) || path.is_empty() {
        file.to_string()
    } else {
        format!("{}{}{}", path, PATH_SEP, file)
    }
}

/// `my_fopen` (util.cc:338): `fopen` after `path_parse`.
pub fn my_fopen(file: Option<&str>, mode: &str, home: &str) -> Option<File> {
    let path = path_parse(file, home)?;
    let mut o = std::fs::OpenOptions::new();
    match mode.as_bytes().first().copied()? {
        b'r' => {
            o.read(true);
            if mode.contains('+') {
                o.write(true);
            }
        }
        b'w' => {
            o.write(true).create(true).truncate(true);
            if mode.contains('+') {
                o.read(true);
            }
        }
        b'a' => {
            o.append(true).create(true);
            if mode.contains('+') {
                o.read(true);
            }
        }
        _ => return None,
    }
    o.open(path).ok()
}

/// `my_fclose` (util.cc:355): `-1` for a missing file, `0` on success.
/// Rust closes on drop, so no close error can be reported.
pub fn my_fclose(file: Option<File>) -> i32 {
    match file {
        None => -1,
        Some(f) => {
            drop(f);
            0
        }
    }
}

/// `my_fgets` (util.cc:375): read a line without its newline, expanding
/// tabs and dropping non-printables.  `Err(1)` is the original's
/// "nothing read" failure (which also clears the buffer).
pub fn my_fgets<R: BufRead>(f: &mut R, n: usize) -> Result<String, i32> {
    let mut buf = String::new();
    let mut i = 0usize;
    loop {
        let c = match f.fill_buf() {
            Ok(b) => match b.first() {
                Some(&c) => c,
                None => return if i == 0 { Err(1) } else { Ok(buf) },
            },
            Err(_) => return Err(1),
        };
        f.consume(1);
        if c == b'\r' || c == b'\n' {
            // DOS (\015\012), Mac (\015) or UNIX (\012).
            let other = if c == b'\r' { b'\n' } else { b'\r' };
            let swallow = matches!(f.fill_buf(), Ok(b) if b.first() == Some(&other));
            if swallow {
                f.consume(1);
            }
            return Ok(buf);
        } else if c == b'\t' {
            // Hack -- require room, then append 1-8 spaces.
            if i + 8 >= n {
                break;
            }
            loop {
                buf.push(' ');
                i += 1;
                if i % 8 == 0 {
                    break;
                }
            }
        } else if (0x20..0x7f).contains(&c) {
            buf.push(c as char);
            i += 1;
            if i >= n {
                break;
            }
        }
        // Other control characters are stripped.
    }
    Err(1)
}

/// `fd_kill` (util.cc:456): delete a file (the original ignores the
/// remove result once the path parses).
pub fn fd_kill(file: Option<&str>) -> i32 {
    let Some(path) = file else { return -1 };
    let _ = std::fs::remove_file(path);
    0
}

/// `fd_move` (util.cc:474): rename a file.
pub fn fd_move(file: Option<&str>, what: Option<&str>) -> i32 {
    let (Some(from), Some(to)) = (file, what) else {
        return -1;
    };
    let _ = std::fs::rename(from, to);
    0
}

/// `fd_make` (util.cc:501): create a new write-only file, failing if it
/// already exists.
pub fn fd_make(file: &str, mode: u32) -> Result<File, i32> {
    let mut o = std::fs::OpenOptions::new();
    o.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        o.mode(mode);
    }
    #[cfg(not(unix))]
    let _ = mode;
    o.open(file).map_err(|_| -1)
}

/// `fd_open` (util.cc:521): open an existing file.  The original's `flags`
/// are the OS `open(2)` flags; the port names the read/write pair.
pub fn fd_open(file: &str, read: bool, write: bool) -> Result<File, i32> {
    let mut o = std::fs::OpenOptions::new();
    o.read(read).write(write);
    o.open(file).map_err(|_| -1)
}

/// `fd_seek` (util.cc:538): seek to an absolute position, reporting
/// failure if the result differs.
pub fn fd_seek(f: &mut File, n: u64) -> i32 {
    match f.seek(SeekFrom::Start(n)) {
        Ok(p) if p == n => 0,
        _ => 1,
    }
}

/// `fd_read` (util.cc:562): read exactly `buf.len()` bytes.
pub fn fd_read<R: Read>(f: &mut R, buf: &mut [u8]) -> i32 {
    match f.read_exact(buf) {
        Ok(()) => 0,
        Err(_) => 1,
    }
}

/// `fd_write` (util.cc:595): write the whole buffer.
pub fn fd_write<W: Write>(f: &mut W, buf: &[u8]) -> i32 {
    match f.write_all(buf) {
        Ok(()) => 0,
        Err(_) => 1,
    }
}

/// `fd_close` (util.cc:628): `-1` for a missing descriptor.
pub fn fd_close(f: Option<File>) -> i32 {
    match f {
        None => -1,
        Some(f) => {
            drop(f);
            0
        }
    }
}

// ---------------------------------------------------------------------------
// ASCII <-> printable text
// ---------------------------------------------------------------------------

/// `hexsym` (tables.cc:79), used by both `octify` and `hexify`.
pub const HEXSYM: [char; 16] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F',
];

/// `octify` (util.cc:691): a decimal as a single octal digit.
pub fn octify(i: u32) -> char {
    HEXSYM[(i % 8) as usize]
}

/// `hexify` (util.cc:699): a decimal as a single hex digit.
pub fn hexify(i: u32) -> char {
    HEXSYM[(i % 16) as usize]
}

/// `deoct` (util.cc:708): an octal digit as a decimal.
pub fn deoct(c: char) -> i32 {
    if c.is_ascii_digit() {
        d2i(c)
    } else {
        0
    }
}

/// `dehex` (util.cc:717): a hexadecimal digit as a decimal.
pub fn dehex(c: char) -> i32 {
    if c.is_ascii_digit() {
        d2i(c)
    } else if c.is_ascii_lowercase() {
        a2i(c) + 10
    } else if c.is_ascii_uppercase() {
        a2i(c.to_ascii_lowercase()) + 10
    } else {
        0
    }
}

/// The per-character `T:` macro configuration (files.cc:577): the
/// template with its `&`/`#` placeholders, the modifier character/name
/// pairs and the trigger names with their two keycodes.
#[derive(Clone, Debug)]
pub struct MacroTriggerSet {
    pub template: String,
    pub modifier_chars: String,
    pub modifier_names: Vec<String>,
    pub trigger_names: Vec<String>,
    pub keycodes: [Vec<String>; 2],
}

impl Default for MacroTriggerSet {
    fn default() -> Self {
        MacroTriggerSet {
            template: String::new(),
            modifier_chars: String::new(),
            modifier_names: Vec::new(),
            trigger_names: Vec::new(),
            keycodes: [Vec::new(), Vec::new()],
        }
    }
}

/// `trigger_text_to_ascii` (util.cc:726).  `input` starts at the `[` of
/// a `\[...]` sequence; appends the expanded bytes to `out` and returns
/// how many input bytes the caller must skip.  Without a template the
/// caller's `str++` consumes just the `[`.
pub fn trigger_text_to_ascii(
    out: &mut Vec<u8>,
    set: Option<&MacroTriggerSet>,
    input: &[u8],
) -> usize {
    let Some(set) = set else { return 1 };
    // str++; -- skip the '['
    let mut pos: usize = 1;
    let mut mod_status = vec![false; set.modifier_names.len()];
    let mut shiftstatus = 0usize;
    // Examine modifier keys.  `iequals` (boost) compares the *whole*
    // remaining buffer, so a modifier name only matches when the rest of
    // the string is exactly that name (an original quirk).
    loop {
        let mut found = false;
        for (i, name) in set.modifier_names.iter().enumerate() {
            if !name.is_empty() && input[pos..].eq_ignore_ascii_case(name.as_bytes()) {
                pos += name.len();
                mod_status[i] = true;
                if set.modifier_chars.as_bytes().get(i) == Some(&b'S') {
                    shiftstatus = 1;
                }
                found = true;
                break;
            }
        }
        if !found {
            break;
        }
    }
    // Look for a trigger name (up to 32 bytes, minus the final `]`).
    let mut found: Option<(usize, usize)> = None;
    for (i, name) in set.trigger_names.iter().enumerate() {
        let tstr: &[u8] = &input[pos..input.len().min(pos + 32)];
        let tstr = if tstr.is_empty() { tstr } else { &tstr[..tstr.len() - 1] };
        if tstr.eq_ignore_ascii_case(name.as_bytes()) && input.get(pos + name.len()) == Some(&b']')
        {
            found = Some((i, name.len()));
            break;
        }
    }
    let Some((i, len)) = found else {
        // Invalid trigger name: emit \x1f\r up to the ']'.
        if let Some(k) = input[pos..].iter().position(|&b| b == b']') {
            out.push(31);
            out.push(13);
            return pos + k + 1;
        }
        return 1;
    };
    pos += len;
    out.push(31);
    let key_code = set.keycodes[shiftstatus]
        .get(i)
        .map(String::as_str)
        .unwrap_or("");
    for &ch in set.template.as_bytes() {
        match ch {
            b'&' => {
                for (j, &mc) in set.modifier_chars.as_bytes().iter().enumerate() {
                    if mod_status.get(j) == Some(&true) {
                        out.push(mc);
                    }
                }
            }
            b'#' => out.extend_from_slice(key_code.as_bytes()),
            _ => out.push(ch),
        }
    }
    out.push(13);
    1 + pos
}

/// `text_to_ascii` (util.cc:829), byte form.
pub fn text_to_ascii_set(s: &[u8], set: Option<&MacroTriggerSet>) -> Vec<u8> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < s.len() {
        let c = s[i];
        if c == b'\\' {
            i += 1;
            if i >= s.len() {
                // The original reads past the terminator here (UB); stop.
                break;
            }
            match s[i] {
                b'[' => {
                    let consumed = trigger_text_to_ascii(&mut out, set, &s[i..]);
                    i += consumed;
                    continue;
                }
                b'x' => {
                    i += 1;
                    let hi = if i < s.len() { dehex(s[i] as char) } else { 0 };
                    i += 1;
                    let lo = if i < s.len() { dehex(s[i] as char) } else { 0 };
                    out.push(((16 * hi + lo) & 0xff) as u8);
                }
                b'\\' => out.push(b'\\'),
                b'^' => out.push(b'^'),
                b's' => out.push(b' '),
                b'e' => out.push(ESCAPE as u8),
                b'b' => out.push(8),
                b'n' => out.push(b'\n'),
                b'r' => out.push(b'\r'),
                b't' => out.push(b'\t'),
                b'0'..=b'3' => {
                    let base = (s[i] - b'0') as u32 * 64;
                    i += 1;
                    let hi = if i < s.len() { deoct(s[i] as char) as u32 } else { 0 };
                    i += 1;
                    let lo = if i < s.len() { deoct(s[i] as char) as u32 } else { 0 };
                    out.push(((base + 8 * hi + lo) & 0xff) as u8);
                }
                _ => {}
            }
            i += 1;
        } else if c == b'^' {
            i += 1;
            let v = if i < s.len() { s[i] } else { 0 };
            out.push(v & 0o37);
            i += 1;
        } else {
            out.push(c);
            i += 1;
        }
    }
    out
}

/// `text_to_ascii` (util.cc:829).
pub fn text_to_ascii(s: &str) -> String {
    String::from_utf8_lossy(&text_to_ascii_set(s.as_bytes(), None)).into_owned()
}

/// `trigger_ascii_to_text` (util.cc:954): recognises a `\x1f...\r` macro
/// trigger and appends its `\[name]` form.  Returns the number of input
/// bytes consumed on success.
pub fn trigger_ascii_to_text(
    out: &mut Vec<u8>,
    set: Option<&MacroTriggerSet>,
    input: &[u8],
) -> Option<usize> {
    let set = set?;
    let mut pos = 0usize;
    let mut key_code: Vec<u8> = Vec::new();
    for &ch in set.template.as_bytes() {
        match ch {
            b'&' => loop {
                let Some(&c) = input.get(pos) else { break };
                let Some(j) = set
                    .modifier_chars
                    .as_bytes()
                    .iter()
                    .position(|&m| m == c)
                else {
                    break;
                };
                if let Some(name) = set.modifier_names.get(j) {
                    out.extend_from_slice(name.as_bytes());
                }
                pos += 1;
            },
            b'#' => {
                while let Some(&c) = input.get(pos) {
                    if c == 13 {
                        break;
                    }
                    key_code.push(c);
                    pos += 1;
                }
            }
            other => {
                if input.get(pos) != Some(&other) {
                    return None;
                }
                pos += 1;
            }
        }
    }
    if input.get(pos) != Some(&13) {
        return None;
    }
    pos += 1;
    let i = set.trigger_names.iter().enumerate().position(|(i, _)| {
        let k = key_code.as_slice();
        set.keycodes[0]
            .get(i)
            .map(|s| k.eq_ignore_ascii_case(s.as_bytes()))
            .unwrap_or(false)
            || set.keycodes[1]
                .get(i)
                .map(|s| k.eq_ignore_ascii_case(s.as_bytes()))
                .unwrap_or(false)
    })?;
    // Commit the `\[name]` form only after the whole trigger validated.
    out.extend_from_slice(b"\\[");
    if let Some(name) = set.trigger_names.get(i) {
        out.extend_from_slice(name.as_bytes());
    }
    out.push(b']');
    Some(pos)
}

/// `ascii_to_text` (util.cc:1019), byte form.
pub fn ascii_to_text_set(s: &[u8], set: Option<&MacroTriggerSet>) -> Vec<u8> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < s.len() {
        let b = s[i];
        i += 1;
        if b == 31 {
            let mut tmp = Vec::new();
            if let Some(n) = trigger_ascii_to_text(&mut tmp, set, &s[i..]) {
                out.extend_from_slice(&tmp);
                i += n;
            } else {
                out.extend_from_slice(b"^_");
            }
        } else if b == ESCAPE as u8 {
            out.extend_from_slice(b"\\e");
        } else if b == b' ' {
            out.extend_from_slice(b"\\s");
        } else if b == 8 {
            out.extend_from_slice(b"\\b");
        } else if b == b'\t' {
            out.extend_from_slice(b"\\t");
        } else if b == b'\n' {
            out.extend_from_slice(b"\\n");
        } else if b == b'\r' {
            out.extend_from_slice(b"\\r");
        } else if b == b'^' {
            out.extend_from_slice(b"\\^");
        } else if b == b'\\' {
            out.extend_from_slice(b"\\\\");
        } else if b < 32 {
            out.push(b'^');
            out.push(b + 64);
        } else if b < 127 {
            out.push(b);
        } else {
            // The original's "i < 64" octal branch is unreachable for a
            // `byte`; bytes >= 127 always use the hex form.
            out.push(b'\\');
            out.push(b'x');
            out.push(hexify(b as u32 / 16) as u8);
            out.push(hexify((b % 16) as u32) as u8);
        }
    }
    out
}

/// `ascii_to_text` (util.cc:1019).
pub fn ascii_to_text(s: &str) -> String {
    String::from_utf8_lossy(&ascii_to_text_set(s.as_bytes(), None)).into_owned()
}

// ---------------------------------------------------------------------------
// The macro package (util.cc:1108)
// ---------------------------------------------------------------------------

/// `macro__pat`/`macro__act`/`macro__use` (init2.cc:508, util.cc:1121).
pub struct MacroSet {
    pats: Vec<String>,
    acts: Vec<String>,
    use_: [bool; 256],
}

impl Default for MacroSet {
    fn default() -> Self {
        MacroSet {
            pats: Vec::new(),
            acts: Vec::new(),
            use_: [false; 256],
        }
    }
}

impl MacroSet {
    pub fn len(&self) -> usize {
        self.pats.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pats.is_empty()
    }

    pub fn pattern(&self, i: usize) -> Option<&str> {
        self.pats.get(i).map(String::as_str)
    }

    pub fn action(&self, i: usize) -> Option<&str> {
        self.acts.get(i).map(String::as_str)
    }

    fn possible(&self, pat: &str) -> bool {
        self.use_[pat.as_bytes().first().copied().unwrap_or(0) as usize]
    }

    /// `macro_find_exact` (util.cc:1127): the macro whose pattern is
    /// exactly `pat`.
    pub fn find_exact(&self, pat: &str) -> i32 {
        if !self.possible(pat) {
            return -1;
        }
        for (i, p) in self.pats.iter().enumerate() {
            if p == pat {
                return i as i32;
            }
        }
        -1
    }

    /// `macro_find_check` (util.cc:1155): the first macro whose pattern
    /// starts with `pat`.
    pub fn find_check(&self, pat: &str) -> i32 {
        if !self.possible(pat) {
            return -1;
        }
        for (i, p) in self.pats.iter().enumerate() {
            if p.starts_with(pat) {
                return i as i32;
            }
        }
        -1
    }

    /// `macro_find_maybe` (util.cc:1183): like `find_check`, but skipping
    /// a macro that exactly matches.
    pub fn find_maybe(&self, pat: &str) -> i32 {
        if !self.possible(pat) {
            return -1;
        }
        for (i, p) in self.pats.iter().enumerate() {
            if !p.starts_with(pat) {
                continue;
            }
            if p == pat {
                continue;
            }
            return i as i32;
        }
        -1
    }

    /// `macro_find_ready` (util.cc:1214): the longest macro pattern that
    /// `pat` starts with (ties favour the later entry).
    pub fn find_ready(&self, pat: &str) -> i32 {
        if !self.possible(pat) {
            return -1;
        }
        let mut n: i32 = -1;
        let mut s: i32 = -1;
        for (i, p) in self.pats.iter().enumerate() {
            if !pat.starts_with(p) {
                continue;
            }
            let t = p.len() as i32;
            if n >= 0 && s > t {
                continue;
            }
            n = i as i32;
            s = t;
        }
        n
    }

    /// `macro_add` (util.cc:1260): add or replace a macro definition.
    pub fn add(&mut self, pat: Option<&str>, act: Option<&str>) -> i32 {
        let (Some(pat), Some(act)) = (pat, act) else {
            return -1;
        };
        let n = self.find_exact(pat);
        if n >= 0 {
            self.acts[n as usize] = act.to_string();
        } else {
            self.pats.push(pat.to_string());
            self.acts.push(act.to_string());
        }
        self.use_[pat.as_bytes().first().copied().unwrap_or(0) as usize] = true;
        0
    }
}

// ---------------------------------------------------------------------------
// Screen writes (the port renders through ECS; these are the row ops)
// ---------------------------------------------------------------------------

/// `c_prt` (util.cc:2127) / `prt` (util.cc:2141): erase the row from
/// `col` on, then write `text` there.
pub fn c_prt_into(row: &mut [u8], text: &str, col: usize) {
    if col >= row.len() {
        return;
    }
    row[col..].fill(b' ');
    for (n, b) in text.bytes().enumerate() {
        match row.get_mut(col + n) {
            Some(slot) => *slot = b,
            None => break,
        }
    }
}

/// `prt` (util.cc:2141): `c_prt` with the default (white) attribute.
pub fn prt_into(row: &mut [u8], text: &str, col: usize) {
    c_prt_into(row, text, col);
}

/// `clear_from` (util.cc:2415): erase every row from `row` down.
pub fn clear_from(screen: &mut [Vec<u8>], row: usize) {
    for line in screen.iter_mut().skip(row) {
        line.fill(b' ');
    }
}

/// `pause_line` (util.cc:2831).
pub const PAUSE_LINE: &str = "[Press any key to continue]";

pub fn pause_line(row: &mut [u8]) {
    prt_into(row, "", 0);
    prt_into(row, PAUSE_LINE, 23);
}

/// `complete_command` (util.cc:2439): tab completion over a command
/// table.  `buf` holds `clen` typed characters; the first candidate that
/// starts with them replaces the buffer (up to `mlen`), later ones
/// shorten it to the longest common prefix.  Returns `len(buf) + 1`.
pub fn complete_command(buf: &mut String, clen: usize, mlen: usize, candidates: &[&str]) -> usize {
    let prefix: Vec<u8> = buf.bytes().take(clen).collect();
    let mut gotone = false;
    for cand in candidates {
        if !cand.as_bytes().starts_with(&prefix) {
            continue;
        }
        if !gotone {
            *buf = cand.chars().take(mlen).collect();
            gotone = true;
        } else {
            let mut max = clen;
            while max < mlen {
                match cand.as_bytes().get(max) {
                    None => break,
                    Some(c) => {
                        if buf.as_bytes().get(max) != Some(c) {
                            break;
                        }
                    }
                }
                max += 1;
            }
            if max < mlen {
                buf.truncate(max);
            }
        }
    }
    buf.len() + 1
}

// ---------------------------------------------------------------------------
// askfor_aux / get_string / get_check / get_com / get_quantity
// ---------------------------------------------------------------------------

/// `askfor_aux` (util.cc:2510, completion disabled): the editing state of
/// the text prompt.  `k` is the cursor; the initial `buf` is the default
/// response and is erased by the first typed character.
pub struct AskforAux {
    pub buf: String,
    pub k: usize,
    pub len: usize,
}

impl AskforAux {
    pub fn new(buf: &str, len: usize) -> Self {
        let max = len.saturating_sub(1);
        AskforAux {
            buf: buf.chars().take(max).collect(),
            k: 0,
            len,
        }
    }

    /// Feed a key.  `Some(true)` on RETURN, `Some(false)` on ESCAPE
    /// (which clears the buffer), `None` while editing.
    pub fn key(&mut self, key: char) -> Option<bool> {
        let done;
        match key {
            ESCAPE => {
                self.k = 0;
                done = Some(false);
            }
            '\n' | '\r' => {
                self.k = self.buf.len();
                done = Some(true);
            }
            // The original's '\t' case falls through to the backspace
            // case when completion is disabled.
            '\t' | '\u{7f}' | '\u{8}' => {
                if self.k > 0 {
                    self.k -= 1;
                }
                done = None;
            }
            c if c == ' ' || c.is_ascii_graphic() => {
                if self.k < self.len {
                    self.buf.truncate(self.k);
                    self.buf.push(c);
                    self.k += 1;
                }
                done = None;
            }
            _ => {
                done = None;
            }
        }
        self.buf.truncate(self.k);
        if done == Some(false) {
            self.buf.clear();
        }
        done
    }
}

/// `get_string` (util.cc:2643): prompt for a string, with `buf` as the
/// default answer.  ESCAPE clears `buf` and returns false.
pub fn get_string(
    prompt: &str,
    buf: &mut String,
    len: usize,
    input: &mut dyn FnMut() -> char,
) -> bool {
    let _ = prompt;
    let mut ed = AskforAux::new(buf, len);
    loop {
        if let Some(res) = ed.key(input()) {
            *buf = ed.buf;
            return res;
        }
    }
}

/// `get_check` prompt construction (util.cc:2681).
pub fn get_check_buffer(prompt: &str) -> String {
    let mut s: String = prompt.bytes().take(70).map(|b| b as char).collect();
    s.push_str("[y/n] ");
    s
}

/// `get_check` (util.cc:2671): returns true only for Y/y.  With
/// `quick_messages` the first key answers.
pub fn get_check(prompt: &str, quick_messages: bool, input: &mut dyn FnMut() -> char) -> bool {
    let _ = get_check_buffer(prompt);
    let i = loop {
        let i = input();
        if quick_messages {
            break i;
        }
        if i == ESCAPE {
            break i;
        }
        if "YyNn".contains(i) {
            break i;
        }
    };
    i == 'Y' || i == 'y'
}

/// `get_com` (util.cc:2720): prompt for a key; false on ESCAPE.
pub fn get_com(command: &mut char, input: &mut dyn FnMut() -> char) -> bool {
    *command = input();
    *command != ESCAPE
}

fn atoi(buf: &str) -> i32 {
    let s = buf.trim_start();
    let (neg, s) = if let Some(r) = s.strip_prefix('-') {
        (true, r)
    } else {
        (false, s.strip_prefix('+').unwrap_or(s))
    };
    let mut v: i64 = 0;
    for c in s.chars() {
        if !c.is_ascii_digit() {
            break;
        }
        v = v * 10 + (c as i64 - '0' as i64);
        if v > i32::MAX as i64 {
            v = i32::MAX as i64;
            break;
        }
    }
    if neg {
        (-v) as i32
    } else {
        v as i32
    }
}

/// `REPEAT_MAX` slots (util.cc:3191-3201).
pub struct RepeatBuffer {
    key: [i32; REPEAT_MAX],
    cnt: usize,
    idx: usize,
}

impl Default for RepeatBuffer {
    fn default() -> Self {
        RepeatBuffer {
            key: [0; REPEAT_MAX],
            cnt: 0,
            idx: 0,
        }
    }
}

impl RepeatBuffer {
    /// `repeat_push` (util.cc:3203).
    pub fn push(&mut self, what: i32) {
        if self.cnt == REPEAT_MAX {
            return;
        }
        self.key[self.cnt] = what;
        self.cnt += 1;
        self.idx += 1;
    }

    /// `repeat_pull` (util.cc:3216).
    pub fn pull(&mut self) -> Option<i32> {
        if self.idx == self.cnt {
            return None;
        }
        let what = self.key[self.idx];
        self.idx += 1;
        Some(what)
    }

    /// `repeat_check` (util.cc:3228): 'n' repeats, other commands restart
    /// the history.
    pub fn check(&mut self, command: i16) -> i16 {
        if command == ESCAPE as i16
            || command == ' ' as i16
            || command == '\r' as i16
            || command == '\n' as i16
        {
            return command;
        }
        if command == 'n' as i16 {
            self.idx = 0;
            if let Some(what) = self.pull() {
                return what as i16;
            }
            return command;
        }
        self.cnt = 0;
        self.idx = 0;
        self.push(command as i32);
        command
    }
}

/// `get_quantity`'s string parsing (util.cc:2808): a letter means "all".
pub fn quantity_parse(buf: &str, max: i32) -> i32 {
    let mut amt = atoi(buf);
    if buf.chars().next().map(|c| c.is_ascii_alphabetic()).unwrap_or(false) {
        amt = max;
    }
    if amt > max {
        amt = max;
    }
    if amt < 0 {
        amt = 0;
    }
    amt
}

/// `get_quantity` (util.cc:2747).  `command_arg` is the pending count,
/// `input` is what `get_string` would return (`None` = cancelled).
pub fn get_quantity(
    prompt: Option<&str>,
    max: i32,
    command_arg: &mut i32,
    repeat: &mut RepeatBuffer,
    input: Option<&str>,
) -> i32 {
    // Use "command_arg".
    if *command_arg != 0 {
        let mut amt = *command_arg;
        *command_arg = 0;
        if amt > max {
            amt = max;
        }
        return amt;
    }
    // Get the item index.
    if max != 1 {
        if let Some(mut amt) = repeat.pull() {
            if amt > max {
                amt = max;
            }
            if amt < 0 {
                amt = 0;
            }
            return amt;
        }
    }
    let _ = prompt;
    let Some(buf) = input else { return 0 };
    let amt = quantity_parse(buf, max);
    if amt != 0 {
        repeat.push(amt);
    }
    amt
}

// ---------------------------------------------------------------------------
// request_command
// ---------------------------------------------------------------------------

/// `request_command` (util.cc:2878): get a command, honouring counts
/// ('0'), control chars ('^'), keymap bypass ('\\'), keymaps and the
/// `^x`/`^*` inscription preventions.  `inkey_next` carries a keymap's
/// expansion into the following calls (as the original's global does).
pub struct RequestCommandCtx<'a> {
    pub mode: usize,
    pub keymap: [[Option<&'a str>; 256]; 2],
    pub ignore_keymaps: Vec<u8>,
    pub always_repeat: bool,
    pub command_new: i16,
    pub inkey_next: VecDeque<u8>,
    pub bypass_keymaps: bool,
    pub inven_mode: bool,
}

impl<'a> RequestCommandCtx<'a> {
    pub fn new(mode: usize) -> Self {
        RequestCommandCtx {
            mode,
            keymap: [[None; 256]; 2],
            ignore_keymaps: Vec::new(),
            always_repeat: false,
            command_new: 0,
            inkey_next: VecDeque::new(),
            bypass_keymaps: false,
            inven_mode: false,
        }
    }

    fn next_key(&mut self, input: &mut dyn FnMut() -> char) -> char {
        if let Some(c) = self.inkey_next.pop_front() {
            return c as char;
        }
        // inkey_real forgets the pointer once it is exhausted.
        self.bypass_keymaps = false;
        input()
    }

    pub fn request_command(
        &mut self,
        shopping: bool,
        input: &mut dyn FnMut() -> char,
        confirm: &mut dyn FnMut(&str) -> bool,
        inscriptions: &[String],
    ) -> (i16, i32, i32) {
        let mut command_arg: i32 = 0;
        let mut command_cmd: i16;
        loop {
            // Hack -- auto-commands.
            let mut cmd: i16 = if self.command_new != 0 {
                let c = self.command_new;
                self.command_new = 0;
                if self.inkey_next.is_empty() && !self.inven_mode {
                    self.bypass_keymaps = true;
                }
                self.inven_mode = false;
                c
            } else {
                self.next_key(input) as i16
            };

            // Command Count.
            if cmd == '0' as i16 {
                let old_arg = command_arg;
                let mut n = NumberInput::new(0, 9999);
                let key = loop {
                    let k = self.next_key(input);
                    if let Some((res, key)) = n.key(k) {
                        command_arg = res as i32;
                        break key;
                    }
                };
                cmd = key as i16;
                // Hack -- handle "zero".
                if command_arg == 0 {
                    command_arg = 99;
                }
                // Hack -- handle "old_arg".
                if old_arg != 0 {
                    command_arg = old_arg;
                }
                // White-space means "enter command now".
                if cmd == ' ' as i16 || cmd == '\n' as i16 || cmd == '\r' as i16 {
                    let mut cmd_char = '\0';
                    if get_com(&mut cmd_char, &mut || self.next_key(input)) {
                        cmd = cmd_char as i16;
                    } else {
                        command_arg = 0;
                        continue;
                    }
                }
            }

            // Allow "keymaps" to be bypassed.
            if cmd == '\\' as i16 {
                let mut cmd_char = '\0';
                get_com(&mut cmd_char, &mut || self.next_key(input));
                cmd = cmd_char as i16;
                self.bypass_keymaps = true;
            }

            // Allow "control chars" to be entered.
            if cmd == '^' as i16 {
                let mut cmd_char = '\0';
                if get_com(&mut cmd_char, &mut || self.next_key(input)) {
                    cmd = ktrl(cmd_char) as i16;
                } else {
                    cmd = 0;
                }
            }

            // Look up applicable keymap.
            let mut act: Option<&'a str> = if (0..256).contains(&(cmd as i32)) {
                self.keymap
                    .get(self.mode)
                    .and_then(|row| row[cmd as usize])
            } else {
                None
            };

            // Mega-Hack -- ignore certain keymaps.
            if shopping && cmd > 0 {
                for &ig in &self.ignore_keymaps {
                    if cmd == ig as i16 {
                        act = None;
                        break;
                    }
                }
            }

            // Apply keymap if not inside a keymap already.
            if let Some(act) = act {
                if self.inkey_next.is_empty() && !self.bypass_keymaps {
                    self.inkey_next = act.bytes().collect();
                    continue;
                }
            }

            // Paranoia.
            if cmd == 0 {
                continue;
            }

            command_cmd = cmd;
            break;
        }

        // Hack -- auto-repeat certain commands.
        if self.always_repeat && command_arg <= 0 && AUTO_REPEAT_CMDS.contains(command_cmd as u8 as char)
        {
            command_arg = 99;
        }

        // Hack -- scan equipment inscriptions.
        for ins in inscriptions {
            let bytes = ins.as_bytes();
            let mut from = 0usize;
            while let Some(rel) = ins[from..].find('^') {
                let p = from + rel;
                if bytes.get(p + 1) == Some(&(command_cmd as u8)) || bytes.get(p + 1) == Some(&b'*')
                {
                    if !confirm("Are you sure? ") {
                        command_cmd = ' ' as i16;
                    }
                }
                from = p + 1;
            }
        }

        (command_cmd, command_arg, 0)
    }
}

// ---------------------------------------------------------------------------
// Misc set-up helpers
// ---------------------------------------------------------------------------

/// `is_a_vowel` (util.cc:3118).
pub fn is_a_vowel(ch: char) -> bool {
    matches!(ch, 'a' | 'e' | 'i' | 'o' | 'u' | 'A' | 'E' | 'I' | 'O' | 'U')
}

/// `get_keymap_dir` (util.cc:3147): a digit key is its own direction, a
/// keymap action contributes its last embedded digit, and 5 means none.
pub fn get_keymap_dir(ch: char, mode: usize, keymap: &dyn Fn(usize, char) -> Option<String>) -> i32 {
    let mut d = 0;
    if ch.is_ascii_digit() {
        d = d2i(ch);
    } else if let Some(act) = keymap(mode, ch) {
        for c in act.chars() {
            if c.is_ascii_digit() {
                d = d2i(c);
            }
        }
    }
    if d == 5 {
        d = 0;
    }
    d
}

/// `strlower` (util.cc:3303): the lowered string (the original only
/// touches the first 256 characters).
pub fn strlower(s: &str) -> String {
    s.chars()
        .enumerate()
        .map(|(i, c)| if i < 256 { c.to_ascii_lowercase() } else { c })
        .collect()
}

/// `test_monster_name` (util.cc:3318): the r_info index of an exact
/// case-insensitive name, else 0.
pub fn test_monster_name(gd: &crate::data::GameData, name: &str) -> usize {
    for (i, m) in gd.monsters.iter().enumerate() {
        if !m.name.is_empty() && m.name.eq_ignore_ascii_case(name) {
            return i;
        }
    }
    0
}

/// `test_mego_name` (util.cc:3333): the re_info index of an exact
/// case-insensitive name, else 0.
pub fn test_mego_name(gd: &crate::data::GameData, needle: &str) -> usize {
    for (i, e) in gd.monster_egos.iter().enumerate() {
        if !e.name.is_empty() && e.name.eq_ignore_ascii_case(needle) {
            return i;
        }
    }
    0
}

/// `test_item_name` (util.cc:3354): the k_info index (object id) of an
/// exact case-insensitive name, else -1.
pub fn test_item_name(gd: &crate::data::GameData, needle: &str) -> i32 {
    for o in &gd.objects {
        if o.name.eq_ignore_ascii_case(needle) {
            return o.id as i32;
        }
    }
    -1
}

// `defines.hpp:246-250`.
pub const DAY: i32 = 11520;
pub const YEAR: i32 = DAY * 365;
pub const HOUR: i32 = DAY / 24;
pub const MINUTE: i32 = HOUR / 60;
pub const DAY_START: i32 = HOUR * 6;

/// `bst` (util.cc:3372): break scalar time.
pub fn bst(what: i32, t: i32) -> i32 {
    let turns = t + (10 * DAY_START);
    match what {
        MINUTE => (turns / 10 / MINUTE) % 60,
        HOUR => (turns / 10 / HOUR) % 24,
        DAY => (turns / 10 / DAY) % 365,
        YEAR => turns / 10 / YEAR,
        _ => 0,
    }
}

/// `get_day` (util.cc:3391): an English ordinal day number.
pub fn get_day(day_no: i32) -> String {
    let day = day_no.to_string();
    let suffix = if day_no / 10 == 1 {
        "th"
    } else if day_no % 10 == 1 {
        "st"
    } else if day_no % 10 == 2 {
        "nd"
    } else if day_no % 10 == 3 {
        "rd"
    } else {
        "th"
    };
    format!("{}{}", day, suffix)
}

/// `ask_menu`'s key step (util.cc:3443): 20 items per page, +/- scroll,
/// a lowercased letter selects within the page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AskMenuStep {
    Cancel,
    Redraw,
    Selected(usize),
}

pub const ASK_MENU_PAGE: usize = 20;

pub fn ask_menu_key(start: &mut usize, c: char, size: usize) -> AskMenuStep {
    if c == ESCAPE {
        return AskMenuStep::Cancel;
    }
    if c == '+' {
        if *start + ASK_MENU_PAGE < size {
            *start += ASK_MENU_PAGE;
        }
        return AskMenuStep::Redraw;
    }
    if c == '-' {
        *start = start.saturating_sub(ASK_MENU_PAGE);
        return AskMenuStep::Redraw;
    }
    let c = c.to_ascii_lowercase();
    let idx = a2i(c) + *start as i32;
    if idx >= size as i32 || idx < 0 {
        return AskMenuStep::Redraw;
    }
    AskMenuStep::Selected(idx as usize)
}

/// `input_box_auto` geometry (util.cc:3563): the box is centred, its
/// half-width is the larger of the prompt and `max`, plus one.
pub fn input_box_geometry(wid: i32, hgt: i32, prompt: &str, max: usize) -> (i32, i32, usize) {
    let y = hgt / 2;
    let x = wid / 2;
    let smax = prompt.len().max(max) + 1;
    (y, x, smax)
}

/// `input_box_auto` (util.cc:3561): a centred one-line text box.
pub fn input_box_auto(
    prompt: &str,
    buf: &mut String,
    max: usize,
    wid: i32,
    hgt: i32,
    input: &mut dyn FnMut() -> char,
) -> bool {
    let _ = input_box_geometry(wid, hgt, prompt, max);
    get_string(prompt, buf, max, input)
}

/// `input_box_auto` (util.cc:3578): the value-returning overload.
pub fn input_box_auto_title(
    title: &str,
    max: usize,
    wid: i32,
    hgt: i32,
    input: &mut dyn FnMut() -> char,
) -> String {
    let mut buf = String::new();
    let _ = input_box_auto(title, &mut buf, max, wid, hgt, input);
    buf
}

/// `timer_type` (timer_type.hpp) plus the collection `new_timer`
/// (util.cc:3601) appends to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Timer {
    pub enabled: bool,
    pub delay: i32,
    pub countdown: i32,
}

impl Timer {
    pub fn new(delay: i32) -> Self {
        Timer {
            enabled: false,
            delay,
            countdown: delay,
        }
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn set_delay_and_reset(&mut self, delay: i32) {
        self.delay = delay;
        self.countdown = delay;
    }

    /// Count down; fires (and resets to `delay`) when it reaches zero.
    pub fn count_down(&mut self, fire: &mut impl FnMut()) -> bool {
        if !self.enabled {
            return false;
        }
        self.countdown -= 1;
        if self.countdown <= 0 {
            self.countdown = self.delay;
            fire();
            return true;
        }
        false
    }
}

/// `new_timer` (util.cc:3601): append a disabled timer, returning its
/// index (the original returns the `timer_type*`).
pub fn new_timer(timers: &mut Vec<Timer>, delay: i32) -> usize {
    timers.push(Timer::new(delay));
    timers.len() - 1
}

/// `get_keymap_mode` (util.cc:3610).
pub const KEYMAP_MODE_ORIG: usize = 0;
pub const KEYMAP_MODE_ROGUE: usize = 1;

pub fn get_keymap_mode(rogue_like_commands: bool) -> usize {
    if rogue_like_commands {
        KEYMAP_MODE_ROGUE
    } else {
        KEYMAP_MODE_ORIG
    }
}

// ---------------------------------------------------------------------------
// Map bounds (the original's cur_hgt/cur_wid and panel window)
// ---------------------------------------------------------------------------

/// `in_bounds` (util.cc:3625): fully inside the outer walls.
pub fn in_bounds(y: i32, x: i32, hgt: i32, wid: i32) -> bool {
    (y > 0) && (x > 0) && (y < hgt - 1) && (x < wid - 1)
}

/// `in_bounds2` (util.cc:3633): on or inside the outer walls.
pub fn in_bounds2(y: i32, x: i32, hgt: i32, wid: i32) -> bool {
    (y >= 0) && (x >= 0) && (y < hgt) && (x < wid)
}

/// `panel_contains` (util.cc:3642): currently on screen.
pub fn panel_contains(
    y: i32,
    x: i32,
    panel_row_min: i32,
    panel_row_max: i32,
    panel_col_min: i32,
    panel_col_max: i32,
) -> bool {
    (y >= panel_row_min)
        && (y <= panel_row_max)
        && (x >= panel_col_min)
        && (x <= panel_col_max)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::load_game_data;

    fn temp_path(name: &str) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("tome-util-{}-{}", std::process::id(), name));
        let _ = std::fs::remove_file(&p);
        p
    }

    #[test]
    fn util_get_number_matches_the_original_editing() {
        let mut n = NumberInput::new(7, 99);
        assert_eq!(n.initial_display(DisplayOption::Immediate), Some(7));
        assert_eq!(n.initial_display(DisplayOption::Delay), None);
        assert_eq!(n.key('5'), None);
        assert_eq!(n.res, 5);
        assert_eq!(n.key('0'), None);
        assert_eq!(n.res, 50);
        // Beyond max: limited and warned.
        assert_eq!(n.key('0'), None);
        assert_eq!(n.res, 99);
        // Backspace edits.
        assert_eq!(n.key('\u{8}'), None);
        assert_eq!(n.res, 9);
        assert_eq!(n.key('\u{7f}'), None);
        assert_eq!(n.res, 0);
        assert_eq!(n.key('x'), Some((0, 'x')));
        // Escape cancels to zero.
        assert_eq!(n.key(ESCAPE), Some((0, ESCAPE)));
        // u32::MAX has no "max + 1 == 0" limit, but the overflow guard holds.
        let mut big = NumberInput::new(0, u32::MAX);
        for _ in 0..12 {
            big.key('9');
        }
        assert_eq!(big.res, u32::MAX);
    }

    #[test]
    fn util_get_count_reads_a_number() {
        let keys = ['4', '2', 'q'];
        let mut it = keys.into_iter();
        let mut input = || it.next().unwrap();
        assert_eq!(get_count(1, 999, &mut input), 42);
    }

    #[test]
    fn util_user_name_falls_back_to_player() {
        assert_eq!(user_name_from(Some("feng")), "feng");
        assert_eq!(user_name_from(None), "PLAYER");
        assert_eq!(user_name_from(Some("")), "PLAYER");
        assert!(!user_name().is_empty());
    }

    #[test]
    fn util_path_parse_expands_tilde_only_for_the_current_user() {
        assert_eq!(path_parse(None, "/home/feng"), None);
        assert_eq!(path_parse(Some("plain.txt"), "/home/feng").unwrap(), "plain.txt");
        assert_eq!(path_parse(Some("/abs/path"), "/home/feng").unwrap(), "/abs/path");
        assert_eq!(path_parse(Some("~/save.ron"), "/home/feng").unwrap(), "/home/feng/save.ron");
        assert_eq!(path_parse(Some("~"), "/home/feng").unwrap(), "/home/feng");
        // The user name is ignored (the original looks up the current uid).
        assert_eq!(path_parse(Some("~bob/x"), "/home/feng").unwrap(), "/home/feng/x");
    }

    #[test]
    fn util_path_build_appends_or_bypasses() {
        assert_eq!(path_build("/home/feng", "a.txt"), "/home/feng/a.txt");
        assert_eq!(path_build("/home/feng", "/etc/x"), "/etc/x");
        assert_eq!(path_build("/home/feng", "~/x"), "~/x");
        assert_eq!(path_build("", "a.txt"), "a.txt");
    }

    #[test]
    fn util_my_fopen_reads_and_writes_after_path_parse() {
        let p = temp_path("fopen.txt");
        let path = p.to_str().unwrap();
        {
            let mut f = my_fopen(Some(path), "w", "/home/feng").unwrap();
            assert_eq!(fd_write(&mut f, b"hello"), 0);
        }
        let mut f = my_fopen(Some(path), "r", "/home/feng").unwrap();
        let mut s = String::new();
        f.read_to_string(&mut s).unwrap();
        assert_eq!(s, "hello");
        assert!(my_fopen(Some(path), "r", "/home/feng").is_some());
        assert!(my_fopen(Some("/no/such/dir/x"), "r", "/home/feng").is_none());
        assert!(my_fopen(None, "r", "/home/feng").is_none());
        assert!(my_fopen(Some(path), "z", "/home/feng").is_none());
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn util_my_fclose_reports_a_missing_file() {
        assert_eq!(my_fclose(None), -1);
        let p = temp_path("fclose.txt");
        std::fs::write(&p, b"x").unwrap();
        let f = File::open(&p).unwrap();
        assert_eq!(my_fclose(Some(f)), 0);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn util_my_fgets_matches_the_original_line_rules() {
        let data = b"ab\tcd\r\nef\n\x01gh\nabcdef";
        let mut cur = std::io::Cursor::new(&data[..]);
        // Tab at column 2 expands to the next multiple of eight.
        assert_eq!(my_fgets(&mut cur, 64).unwrap(), "ab      cd");
        assert_eq!(my_fgets(&mut cur, 64).unwrap(), "ef");
        // Non-printables are stripped.
        assert_eq!(my_fgets(&mut cur, 64).unwrap(), "gh");
        // EOF after some characters is still a success...
        assert_eq!(my_fgets(&mut cur, 64).unwrap(), "abcdef");
        // ...but EOF with nothing read fails.
        assert_eq!(my_fgets(&mut cur, 64), Err(1));
        // Truncation clears the buffer and fails.
        let mut cur = std::io::Cursor::new(&b"abcdef"[..]);
        assert_eq!(my_fgets(&mut cur, 4), Err(1));
    }

    #[test]
    fn util_fd_kill_removes_a_file() {
        let p = temp_path("kill.txt");
        std::fs::write(&p, b"x").unwrap();
        assert_eq!(fd_kill(None), -1);
        assert_eq!(fd_kill(Some(p.to_str().unwrap())), 0);
        assert!(!p.exists());
    }

    #[test]
    fn util_fd_move_renames_a_file() {
        let a = temp_path("move-a.txt");
        let b = temp_path("move-b.txt");
        std::fs::write(&a, b"x").unwrap();
        assert_eq!(fd_move(None, Some("x")), -1);
        assert_eq!(fd_move(Some(a.to_str().unwrap()), None), -1);
        assert_eq!(fd_move(Some(a.to_str().unwrap()), Some(b.to_str().unwrap())), 0);
        assert!(!a.exists() && b.exists());
        let _ = std::fs::remove_file(&b);
    }

    #[test]
    fn util_fd_make_fails_if_the_file_exists() {
        let p = temp_path("make.txt");
        let f = fd_make(p.to_str().unwrap(), 0o600);
        assert!(f.is_ok());
        drop(f);
        assert!(fd_make(p.to_str().unwrap(), 0o600).is_err());
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn util_fd_open_reads_an_existing_file() {
        let p = temp_path("open.txt");
        std::fs::write(&p, b"data").unwrap();
        let mut f = fd_open(p.to_str().unwrap(), true, false).unwrap();
        let mut s = String::new();
        f.read_to_string(&mut s).unwrap();
        assert_eq!(s, "data");
        assert!(fd_open("/no/such/file", true, false).is_err());
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn util_fd_seek_positions_a_file() {
        let p = temp_path("seek.txt");
        std::fs::write(&p, b"abcdef").unwrap();
        let mut f = File::open(&p).unwrap();
        assert_eq!(fd_seek(&mut f, 3), 0);
        let mut s = String::new();
        f.read_to_string(&mut s).unwrap();
        assert_eq!(s, "def");
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn util_fd_read_requires_the_full_buffer() {
        let mut cur = std::io::Cursor::new(&b"abc"[..]);
        let mut buf = [0u8; 3];
        assert_eq!(fd_read(&mut cur, &mut buf), 0);
        assert_eq!(&buf, b"abc");
        let mut cur = std::io::Cursor::new(&b"ab"[..]);
        let mut buf = [0u8; 3];
        assert_eq!(fd_read(&mut cur, &mut buf), 1);
    }

    #[test]
    fn util_fd_write_writes_the_whole_buffer() {
        let mut out: Vec<u8> = Vec::new();
        assert_eq!(fd_write(&mut out, b"abc"), 0);
        assert_eq!(out, b"abc");
    }

    #[test]
    fn util_fd_close_reports_a_missing_fd() {
        assert_eq!(fd_close(None), -1);
        let p = temp_path("close.txt");
        std::fs::write(&p, b"x").unwrap();
        let f = File::open(&p).unwrap();
        assert_eq!(fd_close(Some(f)), 0);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn util_octify_wraps_mod_eight() {
        assert_eq!(octify(0), '0');
        assert_eq!(octify(7), '7');
        assert_eq!(octify(8), '0');
        assert_eq!(octify(9), '1');
        assert_eq!(octify(10), '2');
    }

    #[test]
    fn util_hexify_wraps_mod_sixteen() {
        assert_eq!(hexify(0), '0');
        assert_eq!(hexify(9), '9');
        assert_eq!(hexify(10), 'A');
        assert_eq!(hexify(15), 'F');
        assert_eq!(hexify(16), '0');
        assert_eq!(hexify(27), 'B');
    }

    #[test]
    fn util_deoct_accepts_digits_only() {
        assert_eq!(deoct('0'), 0);
        assert_eq!(deoct('7'), 7);
        assert_eq!(deoct('8'), 8);
        assert_eq!(deoct('a'), 0);
        assert_eq!(deoct(' '), 0);
    }

    #[test]
    fn util_dehex_accepts_all_hex_digits() {
        assert_eq!(dehex('0'), 0);
        assert_eq!(dehex('9'), 9);
        assert_eq!(dehex('a'), 10);
        assert_eq!(dehex('f'), 15);
        assert_eq!(dehex('A'), 10);
        assert_eq!(dehex('F'), 15);
        assert_eq!(dehex('z'), 35);
        assert_eq!(dehex('%'), 0);
    }

    fn trigger_set() -> MacroTriggerSet {
        // The X11 pref's macro template (pref-x11.prf:23 and :32).
        MacroTriggerSet {
            template: "&_#".to_string(),
            modifier_chars: "NSOM".to_string(),
            modifier_names: vec!["control-".into(), "shift-".into(), "alt-".into(), "mod2-".into()],
            trigger_names: vec!["Escape".into(), "KP_Enter".into()],
            keycodes: [
                vec!["FF1B".into(), "FF8D".into()],
                vec!["FF1B".into(), "FF8D".into()],
            ],
        }
    }

    #[test]
    fn util_trigger_text_to_ascii_expands_without_a_template() {
        // Without macro_template the '[', trigger and ']' are dropped.
        let mut out = Vec::new();
        assert_eq!(trigger_text_to_ascii(&mut out, None, b"[Escape]"), 1);
        assert!(out.is_empty());
        // With a set, \[Escape] becomes \x1f<template>\r.
        let set = trigger_set();
        let mut out = Vec::new();
        let consumed = trigger_text_to_ascii(&mut out, Some(&set), b"[Escape]");
        assert_eq!(consumed, "[Escape]".len());
        assert_eq!(out, b"\x1f_FF1B\r");
        // Trailing text after the ']' breaks the original's 32-byte copy
        // comparison (it trims the *last* character of the remainder), so
        // such a trigger takes the unknown-trigger path.
        let mut out = Vec::new();
        let consumed = trigger_text_to_ascii(&mut out, Some(&set), b"[Escape]rest");
        assert_eq!(consumed, "[Escape]".len());
        assert_eq!(out, b"\x1f\r");
        // An unknown name emits \x1f\r up to the ']'.
        let mut out = Vec::new();
        let consumed = trigger_text_to_ascii(&mut out, Some(&set), b"[bogus]x");
        assert_eq!(consumed, "[bogus]".len());
        assert_eq!(out, b"\x1f\r");
        // Without a ']' the pointers are left alone (consumes only '[').
        let mut out = Vec::new();
        assert_eq!(trigger_text_to_ascii(&mut out, Some(&set), b"[Escape"), 1);
        assert!(out.is_empty());
    }

    #[test]
    fn util_text_to_ascii_decodes_the_escape_forms() {
        assert_eq!(text_to_ascii("plain"), "plain");
        assert_eq!(text_to_ascii("a\\sb"), "a b");
        assert_eq!(text_to_ascii("a\\\\b"), "a\\b");
        assert_eq!(text_to_ascii("a\\^b"), "a^b");
        assert_eq!(text_to_ascii("x\\x41y"), "xAy");
        assert_eq!(text_to_ascii("\\e"), "\u{1b}");
        assert_eq!(text_to_ascii("\\b\\n\\r\\t"), "\u{8}\n\r\t");
        // Three octal digits at most: \101 is 'A', \0101 is \x08 + '1'.
        assert_eq!(text_to_ascii("\\101"), "A");
        assert_eq!(text_to_ascii("\\0101"), "\u{8}1");
        assert_eq!(text_to_ascii("^A"), "\u{1}");
        // \[...] with no macro template drops the opening bracket.
        assert_eq!(text_to_ascii("a\\[escape]b"), "aescape]b");
        let set = trigger_set();
        assert_eq!(
            String::from_utf8_lossy(&text_to_ascii_set(b"a\\[Escape]", Some(&set))),
            "a\u{1f}_FF1B\r"
        );
    }

    #[test]
    fn util_trigger_ascii_to_text_round_trips() {
        let set = trigger_set();
        // `ascii_to_text` consumes the 0x1f lead byte before the template.
        let mut out = Vec::new();
        let n = trigger_ascii_to_text(&mut out, Some(&set), b"_FF1B\rrest").unwrap();
        assert_eq!(n, "_FF1B\r".len());
        assert_eq!(out, b"\\[Escape]");
        // No template: no trigger.
        assert_eq!(
            trigger_ascii_to_text(&mut Vec::new(), None, b"_FF1B\r"),
            None
        );
        // Unknown keycode: no trigger.
        assert_eq!(
            trigger_ascii_to_text(&mut Vec::new(), Some(&set), b"_XX\r"),
            None
        );
        assert_eq!(
            trigger_ascii_to_text(&mut Vec::new(), Some(&set), b"_FF1B"),
            None
        );
    }

    #[test]
    fn util_ascii_to_text_encodes_the_printable_forms() {
        assert_eq!(ascii_to_text("plain"), "plain");
        assert_eq!(ascii_to_text("a b"), "a\\sb");
        assert_eq!(ascii_to_text("a\\b"), "a\\\\b");
        assert_eq!(ascii_to_text("a^b"), "a\\^b");
        assert_eq!(ascii_to_text("\u{1b}"), "\\e");
        assert_eq!(ascii_to_text("\u{8}\t\n\r"), "\\b\\t\\n\\r");
        assert_eq!(ascii_to_text("\u{1}"), "^A");
        // 0x1f without a macro template becomes "^_" and round-trips.
        assert_eq!(ascii_to_text("\u{1f}"), "^_");
        assert_eq!(text_to_ascii("^_"), "\u{1f}");
        // Bytes >= 0x7f use the hex form.
        assert_eq!(ascii_to_text_set(&[0xff], None), b"\\xFF");
        let set = trigger_set();
        assert_eq!(
            String::from_utf8_lossy(&ascii_to_text_set(b"\x1f_FF1B\r", Some(&set))),
            "\\[Escape]"
        );
    }

    #[test]
    fn util_macro_find_exact_matches_only_whole_patterns() {
        let mut m = MacroSet::default();
        assert!(m.is_empty());
        assert_eq!(m.add(Some("a"), Some("A")), 0);
        assert_eq!(m.add(Some("ab"), Some("AB")), 0);
        assert_eq!(m.len(), 2);
        assert_eq!(m.find_exact("a"), 0);
        assert_eq!(m.find_exact("ab"), 1);
        assert_eq!(m.find_exact("abc"), -1);
        assert_eq!(m.find_exact("b"), -1);
    }

    #[test]
    fn util_macro_find_check_matches_prefixes() {
        let mut m = MacroSet::default();
        m.add(Some("ab"), Some("AB"));
        m.add(Some("ac"), Some("AC"));
        assert_eq!(m.find_check("a"), 0);
        assert_eq!(m.find_check("ac"), 1);
        assert_eq!(m.find_check("z"), -1);
    }

    #[test]
    fn util_macro_find_maybe_skips_exact_matches() {
        let mut m = MacroSet::default();
        m.add(Some("a"), Some("A"));
        m.add(Some("ab"), Some("AB"));
        assert_eq!(m.find_maybe("a"), 1);
        assert_eq!(m.find_maybe("ab"), -1);
        assert_eq!(m.find_maybe("z"), -1);
    }

    #[test]
    fn util_macro_find_ready_takes_the_longest() {
        let mut m = MacroSet::default();
        m.add(Some("a"), Some("A"));
        m.add(Some("ab"), Some("AB"));
        assert_eq!(m.find_ready("abc"), 1);
        assert_eq!(m.find_ready("a"), 0);
        assert_eq!(m.find_ready("b"), -1);
        // Ties favour the later entry (the original's `s > t` test).
        m.add(Some("abc"), Some("ABC"));
        assert_eq!(m.find_ready("abc"), 2);
    }

    #[test]
    fn util_macro_add_replaces_or_appends() {
        let mut m = MacroSet::default();
        assert_eq!(m.add(None, Some("x")), -1);
        assert_eq!(m.add(Some("a"), None), -1);
        assert_eq!(m.add(Some("a"), Some("old")), 0);
        assert_eq!(m.add(Some("a"), Some("new")), 0);
        assert_eq!(m.len(), 1);
        assert_eq!(m.action(0), Some("new"));
        assert_eq!(m.pattern(0), Some("a"));
    }

    #[test]
    fn util_prt_clears_the_rest_of_the_row() {
        let mut row = [b'_'; 12];
        prt_into(&mut row, "abc", 2);
        // Only from `col` on is erased (Term_erase(col, row, 255)).
        assert_eq!(&row[..], b"__abc       ");
        // Off-screen columns are ignored.
        let mut row = [b'_'; 4];
        prt_into(&mut row, "abc", 4);
        assert_eq!(&row[..], b"____");
    }

    #[test]
    fn util_clear_from_erases_below() {
        let mut screen = vec![vec![b'x'; 4], vec![b'x'; 4], vec![b'x'; 4]];
        clear_from(&mut screen, 1);
        assert_eq!(screen[0], vec![b'x'; 4]);
        assert_eq!(screen[1], vec![b' '; 4]);
        assert_eq!(screen[2], vec![b' '; 4]);
    }

    #[test]
    fn util_complete_command_shortens_to_the_common_prefix() {
        let mut buf = String::from("he");
        assert_eq!(complete_command(&mut buf, 2, 10, &["hello", "help", "helm"]), 4);
        assert_eq!(buf, "hel");
        let mut buf = String::new();
        assert_eq!(complete_command(&mut buf, 0, 10, &["abc", "abd"]), 3);
        assert_eq!(buf, "ab");
        let mut buf = String::from("z");
        assert_eq!(complete_command(&mut buf, 1, 10, &["abc"]), 2);
        assert_eq!(buf, "z");
    }

    #[test]
    fn util_get_string_edits_like_askfor_aux() {
        let mut it = ['h', 'i', '\n'].into_iter();
        let mut input = || it.next().unwrap();
        let mut buf = String::new();
        assert!(get_string("Name: ", &mut buf, 10, &mut input));
        assert_eq!(buf, "hi");

        // Typing replaces the default answer.
        let mut it = ['x', 'y', '\n'].into_iter();
        let mut input = || it.next().unwrap();
        let mut buf = String::from("default");
        assert!(get_string("Name: ", &mut buf, 10, &mut input));
        assert_eq!(buf, "xy");

        // ESC clears and reports false.
        let mut buf = String::from("keep");
        let mut it = [ESCAPE].into_iter();
        let mut input = || it.next().unwrap();
        assert!(!get_string("Name: ", &mut buf, 10, &mut input));
        assert!(buf.is_empty());

        // Backspace deletes the last typed char; a leading Tab does too
        // (the original's fall-through).
        let mut buf = String::new();
        let mut it = ['a', 'b', '\t', 'c', '\n'].into_iter();
        let mut input = || it.next().unwrap();
        assert!(get_string("Name: ", &mut buf, 10, &mut input));
        assert_eq!(buf, "ac");
    }

    #[test]
    fn util_get_check_accepts_only_yes() {
        assert_eq!(get_check_buffer("Really? "), "Really? [y/n] ");
        assert_eq!(
            get_check_buffer(&"x".repeat(100)).len(),
            "x".repeat(70).len() + "[y/n] ".len()
        );
        let mut it = ['y'].into_iter();
        let mut input = || it.next().unwrap();
        assert!(get_check("?", false, &mut input));
        let mut it = ['N'].into_iter();
        let mut input = || it.next().unwrap();
        assert!(!get_check("?", false, &mut input));
        // Bad keys are ignored until a yes/no.
        let mut it = ['z', 'x', 'Y'].into_iter();
        let mut input = || it.next().unwrap();
        assert!(get_check("?", false, &mut input));
        // ESC is a "no".
        let mut it = [ESCAPE].into_iter();
        let mut input = || it.next().unwrap();
        assert!(!get_check("?", false, &mut input));
        // quick_messages answers on the first key.
        let mut it = ['q'].into_iter();
        let mut input = || it.next().unwrap();
        assert!(!get_check("?", true, &mut input));
    }

    #[test]
    fn util_get_com_reports_escape() {
        let mut it = ['x'].into_iter();
        let mut input = || it.next().unwrap();
        let mut command = '\0';
        assert!(get_com(&mut command, &mut input));
        assert_eq!(command, 'x');
        let mut it = [ESCAPE].into_iter();
        let mut input = || it.next().unwrap();
        assert!(!get_com(&mut command, &mut input));
        assert_eq!(command, ESCAPE);
    }

    #[test]
    fn util_get_quantity_uses_arg_repeat_and_input() {
        // command_arg wins and is cleared, capped at max.
        let mut arg = 30;
        let mut repeat = RepeatBuffer::default();
        assert_eq!(get_quantity(None, 10, &mut arg, &mut repeat, None), 10);
        assert_eq!(arg, 0);

        // The repeat buffer is pulled before prompting.
        let mut repeat = RepeatBuffer::default();
        repeat.push(5);
        repeat.push(3);
        assert_eq!(repeat.check('n' as i16), 5);
        let mut arg = 0;
        assert_eq!(get_quantity(None, 10, &mut arg, &mut repeat, Some("1")), 3);

        // Typed input parses; a letter means "all" (the max).
        let mut arg = 0;
        let mut repeat = RepeatBuffer::default();
        assert_eq!(get_quantity(None, 10, &mut arg, &mut repeat, Some("7")), 7);
        let mut arg = 0;
        let mut repeat = RepeatBuffer::default();
        assert_eq!(get_quantity(None, 10, &mut arg, &mut repeat, Some("a")), 10);
        assert_eq!(quantity_parse("99", 10), 10);
        assert_eq!(quantity_parse("-3", 10), 0);

        // Cancelled input returns 0.
        let mut arg = 0;
        assert_eq!(get_quantity(None, 10, &mut arg, &mut repeat, None), 0);

        // max == 1 bypasses the repeat buffer.
        let mut repeat = RepeatBuffer::default();
        repeat.push(9);
        repeat.push(8);
        assert_eq!(repeat.check('n' as i16), 9);
        let mut arg = 0;
        assert_eq!(get_quantity(None, 1, &mut arg, &mut repeat, Some("1")), 1);
    }

    #[test]
    fn util_pause_line_centres_at_column_23() {
        let mut row = [b'_'; 80];
        pause_line(&mut row);
        assert_eq!(&row[23..23 + PAUSE_LINE.len()], PAUSE_LINE.as_bytes());
        assert_eq!(row[22], b' ');
        assert_eq!(row[23 + PAUSE_LINE.len()], b' ');
    }

    #[test]
    fn util_request_command_handles_counts_macros_and_inscriptions() {
        let mut confirm = |_: &str| true;

        // A plain command.
        let mut ctx = RequestCommandCtx::new(KEYMAP_MODE_ORIG);
        let mut it = ['q'].into_iter();
        let mut input = || it.next().unwrap_or('\0');
        assert_eq!(
            ctx.request_command(false, &mut input, &mut confirm, &[]),
            ('q' as i16, 0, 0)
        );

        // '0' collects a count; the terminating key is the command.
        let mut ctx = RequestCommandCtx::new(KEYMAP_MODE_ORIG);
        let mut it = ['0', '4', '2', 'q'].into_iter();
        let mut input = || it.next().unwrap_or('\0');
        assert_eq!(
            ctx.request_command(false, &mut input, &mut confirm, &[]),
            ('q' as i16, 42, 0)
        );

        // A bare zero count becomes 99 and Enter asks for the command.
        let mut ctx = RequestCommandCtx::new(KEYMAP_MODE_ORIG);
        let mut it = ['0', '\n', 'x'].into_iter();
        let mut input = || it.next().unwrap_or('\0');
        assert_eq!(
            ctx.request_command(false, &mut input, &mut confirm, &[]),
            ('x' as i16, 99, 0)
        );

        // ESC during the count entry clears it.
        let mut ctx = RequestCommandCtx::new(KEYMAP_MODE_ORIG);
        let mut it = ['0', '7', ' ', ESCAPE, 'y'].into_iter();
        let mut input = || it.next().unwrap_or('\0');
        assert_eq!(
            ctx.request_command(false, &mut input, &mut confirm, &[]),
            ('y' as i16, 0, 0)
        );

        // Keymaps expand, and the expansion persists across calls.
        let mut ctx = RequestCommandCtx::new(KEYMAP_MODE_ORIG);
        ctx.keymap[KEYMAP_MODE_ORIG][b'x' as usize] = Some("abc");
        let mut it = ['x'].into_iter();
        let mut input = || it.next().unwrap_or('\0');
        assert_eq!(ctx.request_command(false, &mut input, &mut confirm, &[]).0, 'a' as i16);
        assert_eq!(ctx.request_command(false, &mut input, &mut confirm, &[]).0, 'b' as i16);
        assert_eq!(ctx.request_command(false, &mut input, &mut confirm, &[]).0, 'c' as i16);

        // Backslash bypasses a keymap for the following key.
        let mut ctx = RequestCommandCtx::new(KEYMAP_MODE_ORIG);
        ctx.keymap[KEYMAP_MODE_ORIG][b'x' as usize] = Some("zz");
        let mut it = ['\\', 'x'].into_iter();
        let mut input = || it.next().unwrap_or('\0');
        assert_eq!(ctx.request_command(false, &mut input, &mut confirm, &[]).0, 'x' as i16);

        // Caret controlifies the next key.
        let mut ctx = RequestCommandCtx::new(KEYMAP_MODE_ORIG);
        let mut it = ['^', 'a'].into_iter();
        let mut input = || it.next().unwrap_or('\0');
        assert_eq!(ctx.request_command(false, &mut input, &mut confirm, &[]).0, 1);

        // Shops ignore the keymap for their menu letters.
        let mut ctx = RequestCommandCtx::new(KEYMAP_MODE_ORIG);
        ctx.keymap[KEYMAP_MODE_ORIG][b'a' as usize] = Some("b");
        ctx.ignore_keymaps.push(b'a');
        let mut it = ['a'].into_iter();
        let mut input = || it.next().unwrap_or('\0');
        assert_eq!(ctx.request_command(true, &mut input, &mut confirm, &[]).0, 'a' as i16);

        // Auto-commands are used before any key and bypass the keymap.
        let mut ctx = RequestCommandCtx::new(KEYMAP_MODE_ORIG);
        ctx.command_new = 'q' as i16;
        ctx.keymap[KEYMAP_MODE_ORIG][b'q' as usize] = Some("z");
        let mut it = ['a'].into_iter();
        let mut input = || it.next().unwrap_or('\0');
        assert_eq!(ctx.request_command(false, &mut input, &mut confirm, &[]).0, 'q' as i16);

        // always_repeat gives movement/tunnel commands 99 turns.
        let mut ctx = RequestCommandCtx::new(KEYMAP_MODE_ORIG);
        ctx.always_repeat = true;
        let mut it = ['T'].into_iter();
        let mut input = || it.next().unwrap_or('\0');
        assert_eq!(
            ctx.request_command(false, &mut input, &mut confirm, &[]),
            ('T' as i16, 99, 0)
        );

        // '^x' inscriptions ask before running a command.
        let mut ctx = RequestCommandCtx::new(KEYMAP_MODE_ORIG);
        let mut deny = |_: &str| false;
        let mut it = ['d'].into_iter();
        let mut input = || it.next().unwrap_or('\0');
        assert_eq!(
            ctx.request_command(false, &mut input, &mut deny, &["^d".to_string()]).0,
            ' ' as i16
        );
        let mut ctx = RequestCommandCtx::new(KEYMAP_MODE_ORIG);
        let mut it = ['d'].into_iter();
        let mut input = || it.next().unwrap_or('\0');
        assert_eq!(
            ctx.request_command(false, &mut input, &mut confirm, &["^d".to_string()]).0,
            'd' as i16
        );
        let mut ctx = RequestCommandCtx::new(KEYMAP_MODE_ORIG);
        let mut it = ['d'].into_iter();
        let mut input = || it.next().unwrap_or('\0');
        assert_eq!(
            ctx.request_command(false, &mut input, &mut deny, &["^*".to_string()]).0,
            ' ' as i16
        );
    }

    #[test]
    fn util_is_a_vowel_matches_the_original() {
        for c in ['a', 'e', 'i', 'o', 'u', 'A', 'E', 'I', 'O', 'U'] {
            assert!(is_a_vowel(c), "{}", c);
        }
        for c in ['y', 'Y', 'b', ' ', '0'] {
            assert!(!is_a_vowel(c), "{}", c);
        }
    }

    #[test]
    fn util_get_keymap_dir_uses_the_last_digit() {
        let keymap = |_: usize, ch: char| match ch {
            'a' => Some("1b7".to_string()),
            'b' => Some("5".to_string()),
            _ => None,
        };
        assert_eq!(get_keymap_dir('3', 0, &keymap), 3);
        assert_eq!(get_keymap_dir('a', 0, &keymap), 7);
        assert_eq!(get_keymap_dir('b', 0, &keymap), 0);
        assert_eq!(get_keymap_dir('z', 0, &keymap), 0);
    }

    #[test]
    fn util_repeat_push_fills_and_blocks() {
        let mut r = RepeatBuffer::default();
        r.push(1);
        // Pushed keys are not pullable until 'n' resets the index.
        assert_eq!(r.pull(), None);
        for i in 0..REPEAT_MAX {
            r.push(i as i32);
        }
        // The buffer does not grow past REPEAT_MAX.
        assert_eq!(r.pull(), None);
    }

    #[test]
    fn util_repeat_pull_advances_the_index() {
        let mut r = RepeatBuffer::default();
        r.push(4);
        r.push(9);
        assert_eq!(r.check('n' as i16), 4);
        assert_eq!(r.pull(), Some(9));
        assert_eq!(r.pull(), None);
    }

    #[test]
    fn util_repeat_check_repeats_the_last_command() {
        let mut r = RepeatBuffer::default();
        assert_eq!(r.check('a' as i16), 'a' as i16);
        assert_eq!(r.check('b' as i16), 'b' as i16);
        // Each real command restarts the history, so 'n' repeats the last
        // one ('b').
        assert_eq!(r.check('n' as i16), 'b' as i16);
        assert_eq!(r.check('n' as i16), 'b' as i16);
        // Ignored commands neither save nor repeat.
        assert_eq!(r.check(ESCAPE as i16), ESCAPE as i16);
        assert_eq!(r.check(' ' as i16), ' ' as i16);
        assert_eq!(r.check('\n' as i16), '\n' as i16);
        assert_eq!(r.check('\r' as i16), '\r' as i16);
    }

    #[test]
    fn util_strlower_lowercases_only_the_first_256() {
        assert_eq!(strlower("AbC"), "abc");
        assert_eq!(strlower("a b"), "a b");
        let long: String = "A".repeat(300);
        let lowered = strlower(&long);
        assert_eq!(&lowered[..256], "a".repeat(256));
        assert_eq!(&lowered[256..], "A".repeat(44));
    }

    #[test]
    fn util_test_monster_name_is_case_insensitive_with_zero_fallback() {
        let gd = load_game_data();
        let idx = gd
            .monsters
            .iter()
            .position(|m| m.name == "Scrawny cat")
            .unwrap();
        assert_eq!(test_monster_name(&gd, "Scrawny cat"), idx);
        assert_eq!(test_monster_name(&gd, "SCRAWNY CAT"), idx);
        assert_eq!(test_monster_name(&gd, "no such monster"), 0);
    }

    #[test]
    fn util_test_mego_name_is_case_insensitive_with_zero_fallback() {
        let gd = load_game_data();
        let (idx, ego) = gd
            .monster_egos
            .iter()
            .enumerate()
            .find(|(_, e)| !e.name.is_empty())
            .unwrap();
        let upper = ego.name.to_uppercase();
        assert_eq!(test_mego_name(&gd, &upper), idx);
        assert_eq!(test_mego_name(&gd, "no such ego"), 0);
    }

    #[test]
    fn util_test_item_name_returns_the_object_id() {
        let gd = load_game_data();
        let obj = gd.objects.iter().find(|o| !o.name.is_empty()).unwrap();
        assert_eq!(test_item_name(&gd, &obj.name.to_uppercase()), obj.id as i32);
        assert_eq!(test_item_name(&gd, "no such item"), -1);
    }

    #[test]
    fn util_bst_matches_the_calendar_constants() {
        // Sunrise (06:00) at turn zero.
        assert_eq!(bst(HOUR, 0), 6);
        assert_eq!(bst(MINUTE, 0), 0);
        // 86400 turns in, the day counter reads 1.
        assert_eq!(bst(DAY, 86_400), 1);
        assert_eq!(bst(DAY, 0), 0);
        assert_eq!(bst(YEAR, 86_400), 0);
        assert_eq!(bst(12345, 0), 0);
    }

    #[test]
    fn util_get_day_uses_english_ordinals() {
        assert_eq!(get_day(0), "0th");
        assert_eq!(get_day(1), "1st");
        assert_eq!(get_day(2), "2nd");
        assert_eq!(get_day(3), "3rd");
        assert_eq!(get_day(4), "4th");
        assert_eq!(get_day(11), "11th");
        assert_eq!(get_day(12), "12th");
        assert_eq!(get_day(13), "13th");
        assert_eq!(get_day(21), "21st");
        assert_eq!(get_day(22), "22nd");
        assert_eq!(get_day(23), "23rd");
        assert_eq!(get_day(111), "111st");
    }

    #[test]
    fn util_ask_menu_scrolls_by_twenty() {
        let mut start = 0usize;
        assert_eq!(ask_menu_key(&mut start, '+', 45), AskMenuStep::Redraw);
        assert_eq!(start, 20);
        assert_eq!(ask_menu_key(&mut start, '+', 45), AskMenuStep::Redraw);
        assert_eq!(start, 40);
        // No page past the end.
        assert_eq!(ask_menu_key(&mut start, '+', 45), AskMenuStep::Redraw);
        assert_eq!(start, 40);
        assert_eq!(ask_menu_key(&mut start, '-', 45), AskMenuStep::Redraw);
        assert_eq!(start, 20);
        // A letter selects within the page.
        assert_eq!(ask_menu_key(&mut start, 'A', 45), AskMenuStep::Selected(20));
        assert_eq!(ask_menu_key(&mut start, 'z', 45), AskMenuStep::Redraw);
        assert_eq!(ask_menu_key(&mut start, ESCAPE, 45), AskMenuStep::Cancel);
        let mut start = 40usize;
        assert_eq!(ask_menu_key(&mut start, '-', 10), AskMenuStep::Redraw);
        assert_eq!(start, 20);
    }

    #[test]
    fn util_input_box_geometry_centres_the_box() {
        assert_eq!(input_box_geometry(80, 24, "Name:", 10), (12, 40, 11));
        assert_eq!(input_box_geometry(80, 24, "A very long prompt", 3), (12, 40, 19));
    }

    #[test]
    fn util_input_box_auto_returns_the_edited_text() {
        let mut it = ['h', 'i', '\n'].into_iter();
        let mut input = || it.next().unwrap();
        let mut buf = String::new();
        assert!(input_box_auto("Name:", &mut buf, 10, 80, 24, &mut input));
        assert_eq!(buf, "hi");

        let mut it = ['o', 'k', '\n'].into_iter();
        let mut input = || it.next().unwrap();
        assert_eq!(input_box_auto_title("Name:", 10, 80, 24, &mut input), "ok");
    }

    #[test]
    fn util_new_timer_counts_down_when_enabled() {
        let mut timers: Vec<Timer> = Vec::new();
        let i = new_timer(&mut timers, 3);
        assert_eq!(i, 0);
        assert!(!timers[0].enabled);
        let fired = std::cell::Cell::new(0);
        let mut fire = || fired.set(fired.get() + 1);
        // Disabled timers do not tick.
        assert!(!timers[0].count_down(&mut fire));
        timers[0].enable();
        assert!(!timers[0].count_down(&mut fire));
        assert!(!timers[0].count_down(&mut fire));
        assert!(timers[0].count_down(&mut fire));
        assert_eq!(fired.get(), 1);
        assert_eq!(timers[0].countdown, 3);
        timers[0].set_delay_and_reset(1);
        assert!(timers[0].count_down(&mut fire));
        assert_eq!(fired.get(), 2);
        timers[0].disable();
        assert!(!timers[0].count_down(&mut fire));
        assert_eq!(fired.get(), 2);
    }

    #[test]
    fn util_get_keymap_mode_follows_rogue_like() {
        assert_eq!(get_keymap_mode(false), KEYMAP_MODE_ORIG);
        assert_eq!(get_keymap_mode(true), KEYMAP_MODE_ROGUE);
    }

    #[test]
    fn util_in_bounds_excludes_the_outer_walls() {
        assert!(in_bounds(1, 1, 10, 20));
        assert!(!in_bounds(0, 1, 10, 20));
        assert!(!in_bounds(1, 0, 10, 20));
        assert!(!in_bounds(9, 1, 10, 20));
        assert!(!in_bounds(1, 19, 10, 20));
    }

    #[test]
    fn util_in_bounds2_includes_the_outer_walls() {
        assert!(in_bounds2(0, 0, 10, 20));
        assert!(in_bounds2(9, 19, 10, 20));
        assert!(!in_bounds2(-1, 0, 10, 20));
        assert!(!in_bounds2(0, -1, 10, 20));
        assert!(!in_bounds2(10, 0, 10, 20));
        assert!(!in_bounds2(0, 20, 10, 20));
    }

    #[test]
    fn util_panel_contains_checks_the_panel_window() {
        assert!(panel_contains(5, 5, 0, 10, 0, 10));
        assert!(panel_contains(0, 0, 0, 10, 0, 10));
        assert!(panel_contains(10, 10, 0, 10, 0, 10));
        assert!(!panel_contains(11, 5, 0, 10, 0, 10));
        assert!(!panel_contains(5, 11, 0, 10, 0, 10));
        assert!(!panel_contains(-1, 5, 0, 10, 0, 10));
    }
}
