//! Port of `src/z-util.cc`, the non-varargs parts of `src/z-form.cc`
//! and `variable.cc::get_version_string`.

/// `capitalize` (z-util.cc:11): uppercase the first non-space character.
/// The original skips leading whitespace (`isspace`) and only touches
/// lowercase letters before stopping after the first non-space byte.
pub fn capitalize(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut done = false;
    for ch in s.chars() {
        if !done && !ch.is_whitespace() {
            out.extend(ch.to_uppercase());
            done = true;
        } else {
            out.push(ch);
        }
    }
    out
}

/// `plog` (z-util.cc:40): a labelled warning on stderr.
pub fn plog(str: &str) {
    eprintln!("{}", str);
}

/// `quit` (z-util.cc:62): exit with the original's conventions.  A string
/// starting with '+'/'-' is an exit code, any other string is logged
/// first (and exits -1); `None` exits 0.
pub fn quit(str: Option<&str>) -> ! {
    match str {
        None => std::process::exit(0),
        Some(s) if s.starts_with('+') || s.starts_with('-') => {
            let code: i32 = s.parse().unwrap_or(-1);
            std::process::exit(code);
        }
        Some(s) => {
            plog(s);
            std::process::exit(-1);
        }
    }
}

/// `format` (z-form.cc:634): the growable formatting entry point.  Rust
/// callers normally use `format!`; this is the named equivalent.
pub fn format(args: std::fmt::Arguments<'_>) -> String {
    args.to_string()
}

/// `quit_fmt` (z-form.cc:658): vararg interface to `quit()`.  In Rust the
/// variadic call becomes `quit_fmt(format_args!(...))`.
pub fn quit_fmt(args: std::fmt::Arguments<'_>) -> ! {
    quit(Some(&args.to_string()))
}

/// `count_bits` (util.cc:3291): number of set bits.
pub fn count_bits(x: u32) -> u32 {
    x.count_ones()
}

/// The module version (`variable.cc:600` prints
/// `"<module> <major>.<minor>.<patch><IS_CVS>"`).  The checked-out game is
/// tagged `v2.4.0-ah` with the `(ah, git)` suffix from `defines.hpp`.
pub const VERSION_MAJOR: i32 = 2;
pub const VERSION_MINOR: i32 = 4;
pub const VERSION_PATCH: i32 = 0;
pub const IS_CVS: &str = " (ah, git)";

/// `get_version_string` (variable.cc:600).
pub fn get_version_string() -> String {
    format!(
        "ToME {}.{}.{}{}",
        VERSION_MAJOR, VERSION_MINOR, VERSION_PATCH, IS_CVS
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capitalize_skips_leading_space_and_only_touches_the_first_word() {
        assert_eq!(capitalize("orc"), "Orc");
        assert_eq!(capitalize("  orc"), "  Orc");
        assert_eq!(capitalize("\telf archer"), "\tElf archer");
        assert_eq!(capitalize("Giant"), "Giant");
        assert_eq!(capitalize(""), "");
        assert_eq!(capitalize("3 headed dog"), "3 headed dog");
    }

    #[test]
    fn count_bits_matches_popcount() {
        assert_eq!(count_bits(0), 0);
        assert_eq!(count_bits(0b1011), 3);
        assert_eq!(count_bits(u32::MAX), 32);
    }

    #[test]
    fn version_string_matches_the_checked_out_release() {
        assert_eq!(get_version_string(), "ToME 2.4.0 (ah, git)");
    }

    #[test]
    fn format_and_plog_are_the_named_equivalents() {
        assert_eq!(format(format_args!("{}+{}", 1, 2)), "1+2");
    }
}
