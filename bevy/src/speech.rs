//! Monster speech (files.cc get_rnd_line / get_xtra_line, melee2.cc
//! SPEAK_CHANCE): r_info F:CAN_SPEAK monsters taunt, panic and die with
//! words.  The text files are copied verbatim from lib/file.

use rand::Rng;
use std::collections::HashMap;
use std::sync::OnceLock;

const MONDEATH_TXT: &str = include_str!("../assets/data/mondeath.txt");
const MONSPEAK_TXT: &str = include_str!("../assets/data/monspeak.txt");
const MONFEAR_TXT: &str = include_str!("../assets/data/monfear.txt");
const BRAVADO_TXT: &str = include_str!("../assets/data/bravado.txt");
const SPEAKPET_TXT: &str = include_str!("../assets/data/speakpet.txt");

static MONDEATH: OnceLock<Vec<&'static str>> = OnceLock::new();
static MONSPEAK: OnceLock<HashMap<u32, Xtra>> = OnceLock::new();
static MONFEAR: OnceLock<Vec<&'static str>> = OnceLock::new();
static BRAVADO: OnceLock<Vec<&'static str>> = OnceLock::new();
static SPEAKPET: OnceLock<Vec<&'static str>> = OnceLock::new();

fn lines_of(text: &'static str) -> Vec<&'static str> {
    text.lines().map(str::trim_end).collect()
}

/// files.cc get_rnd_line: the first line holds the number of valid lines;
/// line k (1..=count) is picked and the (k+1)-th line returned, which
/// deliberately skips the "buffer" line at index 1.
fn rnd_line(file: &[&'static str], rng: &mut impl Rng) -> Option<&'static str> {
    let count: usize = file.first()?.trim().parse().ok()?;
    if count == 0 {
        return None;
    }
    let k = rng.gen_range(1..=count);
    file.get(k + 1).copied()
}

/// One unique's own lines ("monspeak.txt"): bravado lines then fear lines.
struct Xtra {
    normal: Vec<&'static str>,
    fear: Vec<&'static str>,
    /// `get_xtra_line` draws `rand_int(count)`; with a zero count that is
    /// the degenerate 0 and the next `my_fgets` returns the raw line at
    /// the cursor instead of falling back to the generic tables.  For a
    /// zero fear list that is the line after the "0" count (r_idx 934's
    /// Fangorn reads the following blank line).
    normal_zero_line: Option<&'static str>,
    fear_zero_line: Option<&'static str>,
}

/// files.cc get_xtra_line parser.  A contiguous block of `N:` lines
/// shares the counts and lines that follow it (monspeak.txt documents
/// this for monsters with identical lines).
fn parse_xtra(text: &'static str) -> HashMap<u32, Xtra> {
    let lines = lines_of(text);
    let mut out = HashMap::new();
    let mut i = 0;
    while i < lines.len() {
        if !lines[i].starts_with("N:") {
            i += 1;
            continue;
        }
        // Collect every index of the contiguous N: block.
        let mut idxs = Vec::new();
        while i < lines.len() && lines[i].starts_with("N:") {
            if let Some((num, _)) = lines[i][2..].split_once(':') {
                if let Ok(idx) = num.trim().parse::<u32>() {
                    idxs.push(idx);
                }
            }
            i += 1;
        }
        // The next non-N line holds the bravado line count.
        if i >= lines.len() {
            break;
        }
        let normal_count: usize = lines[i].trim().parse().unwrap_or(0);
        i += 1;
        let normal_start = i;
        let normal = lines
            .get(i..i + normal_count)
            .map(|s| s.to_vec())
            .unwrap_or_default();
        i += normal_count;
        if i >= lines.len() {
            break;
        }
        let fear_count: usize = lines[i].trim().parse().unwrap_or(0);
        i += 1;
        let fear_start = i;
        let fear = lines
            .get(i..i + fear_count)
            .map(|s| s.to_vec())
            .unwrap_or_default();
        i += fear_count;
        for idx in idxs {
            out.insert(
                idx,
                Xtra {
                    normal: normal.clone(),
                    fear: fear.clone(),
                    normal_zero_line: lines.get(normal_start).copied(),
                    fear_zero_line: lines.get(fear_start).copied(),
                },
            );
        }
    }
    out
}

fn mondeath() -> &'static Vec<&'static str> {
    MONDEATH.get_or_init(|| lines_of(MONDEATH_TXT))
}

fn monspeak() -> &'static HashMap<u32, Xtra> {
    MONSPEAK.get_or_init(|| parse_xtra(MONSPEAK_TXT))
}

fn monfear() -> &'static Vec<&'static str> {
    MONFEAR.get_or_init(|| lines_of(MONFEAR_TXT))
}

fn bravado() -> &'static Vec<&'static str> {
    BRAVADO.get_or_init(|| lines_of(BRAVADO_TXT))
}

fn speakpet() -> &'static Vec<&'static str> {
    SPEAKPET.get_or_init(|| lines_of(SPEAKPET_TXT))
}

/// A dying CAN_SPEAK monster's last words (xtra2.cc monster death).
pub fn death_line(rng: &mut impl Rng) -> Option<&'static str> {
    rnd_line(mondeath(), rng)
}

/// What a monster says on its turn (melee2.cc make_move): its own lines
/// from monspeak.txt if any, otherwise the friendly/fear/bravado tables.
pub fn monster_speech(
    def_idx: usize,
    afraid: bool,
    friendly: bool,
    rng: &mut impl Rng,
) -> Option<&'static str> {
    if let Some(x) = monspeak().get(&(def_idx as u32)) {
        let (list, zero_line) = if afraid {
            (&x.fear, x.fear_zero_line)
        } else {
            (&x.normal, x.normal_zero_line)
        };
        if !list.is_empty() {
            return Some(list[rng.gen_range(0..list.len())]);
        }
        // rand_int(0) keeps the cursor: the C++ returns the raw line that
        // follows the zero count instead of a generic table line.
        if let Some(line) = zero_line {
            return Some(line);
        }
    }
    let file = if friendly {
        speakpet()
    } else if afraid {
        monfear()
    } else {
        bravado()
    };
    rnd_line(file, rng)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn rnd_line_uses_the_count_and_skips_the_buffer_line() {
        const FILE: &str = "3\nBUFFER LINE\nalpha\nbeta\ngamma\n";
        let lines: Vec<&'static str> = FILE.lines().collect();
        let mut rng = StdRng::seed_from_u64(11);
        let mut seen = std::collections::HashSet::new();
        for _ in 0..300 {
            let line = rnd_line(&lines, &mut rng).expect("line");
            assert!(["alpha", "beta", "gamma"].contains(&line));
            seen.insert(line);
        }
        assert_eq!(seen.len(), 3, "every counted line must be reachable");
        // A zero count yields no line at all.
        assert_eq!(rnd_line(&["0", "buffer", "x"], &mut rng), None);
    }

    #[test]
    fn xtra_lines_split_bravado_and_fear() {
        const FILE: &str = "N:8:Farmer Maggot\n2\nbravo one\nbravo two\n1\nfeared\nN:9:Other\n0\n2\nfear a\nfear b\n";
        let parsed = parse_xtra(FILE);
        let maggot = parsed.get(&8).expect("entry 8");
        assert_eq!(maggot.normal, vec!["bravo one", "bravo two"]);
        assert_eq!(maggot.fear, vec!["feared"]);
        let other = parsed.get(&9).expect("entry 9");
        assert!(other.normal.is_empty());
        assert_eq!(other.fear, vec!["fear a", "fear b"]);
        // Contiguous N: lines share the following lists (the file
        // documents this for monsters with identical lines).
        let shared = parse_xtra("N:8:A\nN:9:B\n2\nx\ny\n1\nz\n");
        let a = shared.get(&8).expect("shared 8");
        let b = shared.get(&9).expect("shared 9");
        assert_eq!(a.normal, b.normal);
        assert_eq!(a.fear, b.fear);
        assert_eq!(a.normal, vec!["x", "y"]);
        assert_eq!(a.fear, vec!["z"]);
    }

    #[test]
    fn speech_tables_parse_and_pick_lines() {
        let mut rng = StdRng::seed_from_u64(3);
        // The buffer line (index 1) is never returned.
        for _ in 0..200 {
            let line = death_line(&mut rng).expect("mondeath line");
            assert!(!line.contains("BUFFER LINE"));
            assert!(!line.trim().is_empty());
        }
        // Farmer Maggot (r_info 8) has his own lines: 3 bravado, 2 fear.
        let xtra = monspeak();
        let maggot = xtra.get(&8).expect("Farmer Maggot lines");
        assert_eq!(maggot.normal.len(), 3);
        assert_eq!(maggot.fear.len(), 2);
        // The contiguous N: block at monspeak.txt:111 (180/237) shares
        // its lines; both must be registered.
        let orfax = xtra.get(&180).expect("Orfax lines");
        let boldor = xtra.get(&237).expect("Boldor shares Orfax's lines");
        assert_eq!(orfax.normal, boldor.normal);
        assert_eq!(orfax.fear, boldor.fear);
        assert!(!orfax.normal.is_empty());
        // Unique lines are used before the generic tables; fear switches
        // the list.
        let a = monster_speech(8, false, false, &mut rng).unwrap();
        let b = monster_speech(8, true, false, &mut rng).unwrap();
        assert!(maggot.normal.contains(&a));
        assert!(maggot.fear.contains(&b));
        // A monster without its own lines falls back to bravado.
        let c = monster_speech(9999, false, false, &mut rng).expect("bravado");
        assert!(!bravado().is_empty());
        assert!(bravado().contains(&c) || !c.contains("N:"));
        // Fangorn (r_idx 934) has six bravado lines and a zero fear count:
        // get_xtra_line's rand_int(0) then reads the following raw line
        // (a blank one) instead of a generic monfear line.
        let fangorn = xtra.get(&934).expect("Fangorn lines");
        assert_eq!(fangorn.normal.len(), 6);
        assert!(fangorn.fear.is_empty());
        assert_eq!(fangorn.fear_zero_line, Some(""));
        assert_eq!(monster_speech(934, true, false, &mut rng), Some(""));
    }
}
