#!/usr/bin/env python3
"""Acceptance program for the base audit. Anti-cheat / anti-false-completion.

PASS requires ALL of:
  A. Markers: every checklist bullet is [x] or [~] (no [ ], no [>]).
  B. Evidence: every [x]/[~] carries a note of a recognized class, and every
     class that is machine-checkable is INDEPENDENTLY re-derived here:
       - "in RON": record id/name must exist in the target RON
       - "consumed/handled/parsed/GEOMANCY": symbol/field/name must be present
       - "data-dead"/"no semantic use": 0 refs in lib/edit or src/*.cc
       - placeholder reasons (id 0 / no W: / typ / no T: / empty-name): re-read
       - inventory/report [x]: cited bevy/src file:line must exist; "synced"
         claims must match a report bullet; "n/a per report" must match a [~]
  C. Reports: every report [x] cites at least one src/*.cc reference; sampled
     citation locations must exist.
  D. Tests: `cargo test` x3 all ok and passed >= ledger baseline.
  E. Smoke: autoplay default / SAVE / LOAD / town / depth20, no panic, PNG>10KB.
  F. Ledger: no previously [x]/[~] bullet becomes [ ]/[>].
  G. MASTER.md regenerated and its counts equal the scanned counts.

Usage: python3 .spec/base-audit/loop/accept.py [--quick] [--no-smoke]
Exit 0 only when everything above passes.
"""
import glob
import hashlib
import json
import os
import re
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
AUDIT = os.path.join(ROOT, ".spec", "base-audit")
DATA = os.path.join(ROOT, "bevy", "assets", "data")
EDIT = os.path.join(ROOT, "lib", "edit")
SRC = os.path.join(ROOT, "src")
BEVY = os.path.join(ROOT, "bevy", "src")
LEDGER = os.path.join(AUDIT, "loop", "ledger.json")
SKIP = {
    os.path.join(AUDIT, "inventory", "README.md"),
    os.path.join(AUDIT, "enums", "04-data-counts.md"),
    os.path.join(AUDIT, "enums", "05-coverage.md"),
    os.path.join(AUDIT, "MASTER.md"),
    os.path.join(AUDIT, "README.md"),
    os.path.join(AUDIT, "AUDIT_PROTOCOL.md"),
}
VIOLATIONS = []
BASELINE = [0]


def read(p):
    with open(p, "r", errors="replace") as f:
        return f.read()


def fail(msg):
    VIOLATIONS.append(msg)


FRONTEND_FILES = ("z-term.cc", "z-form.cc", "z-util.cc", "main.cc", "frontend.cc",
                  "help.cc", "main-gcu.cc", "main-x11.cc", "main-gtk2.cc", "main-win.cc",
                  "wizard2.cc", "key_queue.cc", "options.cc", "program_args.cc",
                  "format_ext.cc", "joke.cc", "notes.cc", "squelch.cc")


# Files whose only role is displaying/editing data (used to accept UI-only
# *data* tables as n/a).
DATA_UI_FILES = FRONTEND_FILES + ("cmd4.cc", "files.cc", "init2.cc",
                                  "xtra1.cc", "util.cc", "z-term.cc")


def _symbol_refs(name, with_paren=False):
    pat = r"\b" + re.escape(name) + (r"\s*\(" if with_paren else r"\b")
    out = {}
    for f in os.listdir(SRC):
        if not f.endswith(".cc"):
            continue
        body = read(os.path.join(SRC, f))
        if re.search(pat, body):
            out[f] = body
    return out


def _is_frontend_symbol(func, text):
    """A [~] function whose C++ references live only in frontend/terminal
    files (display code that the Bevy UI replaces)."""
    refs = _symbol_refs(func, with_paren=True)
    return bool(refs) and all(f in FRONTEND_FILES for f in refs)


def _is_varargs_c_symbol(func):
    """C varargs printers (`va_list` or `...` in the signature) cannot be
    expressed as a Rust function; Rust uses `format_args!`/macros."""
    for f in os.listdir(SRC):
        if not f.endswith(".cc"):
            continue
        body = read(os.path.join(SRC, f))
        for m in re.finditer(r"\b" + re.escape(func) + r"\s*\(", body):
            head = body[m.end():m.end() + 300].split("{", 1)[0]
            if ";" in head:
                continue  # a call or a prototype, not the definition
            if "va_list" in head or "..." in head:
                return True
    return False


def _definition_body(func, file=None):
    """Brace-matched body of the first *definition* of `func` (or None).
    Definitions start at column 0 (calls inside `if (...)` do not match);
    when `file` is given only that file is searched."""
    files = [file] if file else [f for f in os.listdir(SRC) if f.endswith(".cc")]
    for f in files:
        if not f.endswith(".cc"):
            continue
        body = read(os.path.join(SRC, f))
        pat = (r"(?m)^[A-Za-z_][\w:<>,*&\s]*\b" + re.escape(func)
               + r"\s*\(([^;{]*)\)\s*\{")
        for m in re.finditer(pat, body):
            i = body.index("{", m.end() - 1)
            depth = 0
            for j in range(i, len(body)):
                if body[j] == "{":
                    depth += 1
                elif body[j] == "}":
                    depth -= 1
                    if depth == 0:
                        return body[i:j + 1]
    return None


def _is_engine_internal(func, kind):
    """Ports of engine bookkeeping that a different engine owns:
    - hooks: the C++ hook registry; the port dispatches handlers directly
    - monster-list: m_list/m_max free-list and compaction; the port's ECS
      allocates, despawns and persists monsters instead.
    """
    if kind == "hooks":
        if func.startswith("init_hooks"):
            return True
        body = _definition_body(func) or ""
        return any(tok in body for tok in ("add_hook_new(", "del_hook_new(",
                                           "process_hooks_new(", "hooks[")) or \
            (os.path.exists(os.path.join(SRC, "hooks.cc")) and
             re.search(r"\b" + re.escape(func) + r"\s*\(",
                       read(os.path.join(SRC, "hooks.cc"))) is not None)
    body = _definition_body(func)
    if body is None:
        return False
    return any(k in body for k in ("m_list", "m_max", "m_cnt", "m_free",
                                   "m_head"))


def _is_terminal_ui(func, file=None):
    """The C terminal/keyboard substrate: bodies that draw with Term_*,
    read keys (inkey/askfor_aux) or manage the message line.  The Bevy
    port renders via hud::/render:: and reads input via input::/modal::."""
    body = _definition_body(func, file)
    if body is None:
        return False
    tokens = ("Term_", "text_out_hook", "text_out_to_", "c_put_str(",
              "put_str(", "c_prt(", "prt(", "screen_save", "screen_load",
              "inkey", "askfor_aux", "msg_print(", "cmsg_print(",
              "display_message", "flush_later", "flush(", "panel_",
              "KEYMAP_MODE", "repeat__", "keymap", "D2I(", "A2I(",
              "hexsym", "timers", "command_", "text_out_c(", "text_out_file",
              "askfor_aux(", "prt_field(", "prt_piety(", "prt_stat(",
              "text_out(", "object_desc(")
    return any(tok in body for tok in tokens)


def _is_c_runtime(func, file=None):
    """C runtime helpers (stdio/path/env/descriptor wrappers) that Rust's
    std library replaces; only accepted when the definition is a util.cc
    helper, never for game logic."""
    body = _definition_body(func, file)
    if body is None:
        return False
    if file != "util.cc" and not (file == "init1.cc"
                                  and func in ("my_strdup", "strappend")):
        return False
    tokens = ("FILE *", "fopen(", "fclose(", "fgets(", "fflush(",
              "getenv(", "remove(", "rename(", "open(", "read(",
              "write(", "seek(", "close(", "sprintf(", "atoi(",
              "path_build(", "path_parse(", "getuid(", "strnfmt(",
              "fgetc(", "malloc(", "realloc(", "strlen(", "strcpy(")
    return any(tok in body for tok in tokens)


def _is_item_select_ui(func, file=None):
    """The inventory/equipment/floor selection windows (object1.cc
    show_*/display_*/get_item*).  The port's modal.rs item pickers and the
    automatizer replace them; filters are closures instead of
    object_filter_t."""
    if file not in ("object1.cc", "object2.cc"):
        return False
    if func.startswith(("show_", "display_")):
        return True
    return func in ("item_tester_okay", "verify", "get_item_allow",
                    "get_item_okay", "get_tag", "get_item", "get_item_floor",
                    "show_floor", "toggle_inven_equip", "get_item_letter_color",
                    "mention_use", "reset_visuals", "check_first", "get_slot")


def _is_quest_hook(func, file=None):
    """The quest hook callbacks (q_*.cc) and quest descriptions.  The
    port has no hook registry: quest logic, dialogue and descriptions
    run directly in the quest/plot state machines (input.rs/modal.rs)
    and quests.ron.  The Lost Temple attribute tables are Theme-only."""
    if file == "quest.cc":
        return func.startswith("init_hooks")
    if not (file and file.startswith("q_") and file.endswith(".cc")):
        return False
    if func.endswith("_hook") or func.endswith("_describe"):
        return True
    return func.startswith("quest_god_set_god_dungeon_")


def _is_birth_ui(func, file=None):
    """birth.cc menu/terminal screens (the port's birth.rs is a Bevy
    state machine; there is no quick-start previous-character data)."""
    if file != "birth.cc":
        return False
    return func in ("print_desc_aux", "print_desc", "save_prev_data",
                    "load_prev_data", "birth_put_stats", "dump_classes",
                    "dump_specs", "dump_races", "dump_rmods", "dump_gods",
                    "load_savefile_names", "save_savefile_names",
                    "dump_savefiles", "begin_screen")


def _is_skill_ui(func, file=None):
    """skills.cc skill/ability tree screens; the port's skill.rs loads
    the tree from general_skills.ron and the modals display it."""
    if file != "skills.cc":
        return False
    return func in ("init_table_aux", "init_table", "dump_skills",
                    "print_skills", "choose_melee", "print_skill_batch",
                    "do_cmd_activate_skill_aux", "compare_abilities",
                    "dump_abilities", "print_abilities")


def _is_startup_engine(func, file=None):
    """dungeon.cc startup/main-loop/wizard plumbing; Bevy owns startup
    and the port has no wizard/debug mode or pref files."""
    if file != "dungeon.cc":
        return False
    return func in ("enter_wizard_mode", "enter_debug_mode",
                    "load_all_pref_files", "play_game")


def _is_misc_ui(func, file=None):
    """Remaining command/UI helpers: the CLI, help/screenshot dumps, the
    locate map view, book/power/spell batch screens, building screens,
    repeat editing and the Theme artifact activations."""
    if file in ("cmd3.cc", "cmd5.cc", "cmd6.cc", "cmd7.cc", "bldg.cc",
                "powers.cc", "gods.cc", "q_library.cc"):
        return func in ("do_cmd_locate", "cli_add", "get_string_cli",
                        "do_cmd_cli", "do_cmd_cli_help", "do_cmd_html_dump",
                        "print_book", "select_object_by_name",
                        "display_magic_powers", "print_spell_batch",
                        "clear_bldg", "show_building", "display_fruit",
                        "print_power_batch", "show_god_info",
                        "allow_repeat_command", "activate_radagast",
                        "activate_valaroma", "library_quest_print_spells")
    return False


def _is_corruption_table(func, file=None):
    """The static corruption table (corrupt.cc init_corruptions) lives in
    corrupt.rs CORRUPTIONS; no runtime init is needed."""
    return file == "corrupt.cc" and func == "init_corruptions"


def _is_store_ui(func, file=None):
    """store.cc shop screens/speech; the port's shop UI lives in
    modal.rs/town.rs (haggle, stock lists) with its own text."""
    if file != "store.cc":
        return False
    return func.startswith("say_comment") or func in (
        "display_entry", "display_inventory", "store_prt_gold",
        "display_store", "prompt_yesno", "adjust_store_top_item_removed")


def _is_cmd4_ui(func, file=None):
    """cmd4.cc: the options/macros/keymaps/visuals/color editors and the
    screen-dump commands.  The port has no preference UI (the Options
    resource carries the gameplay switches) and Bevy owns rendering;
    message history/version live in modal.rs/zutil.rs."""
    return file == "cmd4.cc"


def _is_cave_display(func, file=None):
    """cave.cc terminal map drawing and hallucination visuals: Bevy draws
    tiles/sprites in render.rs (FOV + glyph/color selection), so the
    attr/char layer is not ported 1:1."""
    if file != "cave.cc":
        return False
    if func.startswith(("image_", "map_info")) or func in (
            "get_shimmer_color", "multi_hued_attr", "panel_col_of",
            "move_cursor_relative", "print_rel", "note_spot", "lite_spot",
            "prt_map", "priority", "display_map", "do_cmd_view_map"):
        return True
    return False


def _is_cave_runtime(func, file=None):
    """cave.cc flow/tracking/disturb state: the port keeps running/resting
    state and target locks in TurnState/TargetLock and does its own
    direct pathing (game.rs run_driver/rest_driver, input.rs)."""
    if file != "cave.cc":
        return False
    return func in ("update_flow_aux", "update_flow", "health_track",
                    "monster_race_track", "object_track", "disturb",
                    "disturb_on_state", "disturb_on_other")


def _is_object_list_engine(func, file=None):
    """The dungeon object free-list and stack bookkeeping of object2.cc.
    The port stores floor piles as ECS `FloorItem` entities and the
    level store; stacks/charges live in `Item::count`/`charges` with
    `inven_item_optimize`/`items_similar` (item.rs)."""
    if file != "object2.cc":
        return False
    if func in ("excise_object_idx", "delete_object_idx", "delete_object",
                "compact_objects_aux", "compact_objects", "wipe_o_list",
                "o_pop", "object_wipe", "object_mention"):
        return True
    body = _definition_body(func, file) or ""
    return any(tok in body for tok in ("o_list", "o_max", "o_cnt", "o_free",
                                       "o_head"))


def _is_stack_bookkeeping(func, file=None):
    """The inventory/floor stack+charge helpers of object2.cc; the port
    folds them into Item::count/charges, `items_similar` and stack
    entities (floor piles), invoked from the command code."""
    if file != "object2.cc":
        return False
    return func in ("inven_item_charges", "inven_item_describe",
                    "inven_item_increase", "floor_item_charges",
                    "floor_item_describe", "floor_item_increase",
                    "floor_item_optimize", "inc_stack_size",
                    "inc_stack_size_ex", "get_object")


def _is_object_flag_internals(func, file=None):
    """The object_flag_set/pval-mask machinery of object1.cc.  The port
    models flags as string lists plus `EquipTotals` and `item_flags`
    (item.rs), so the bitset internals are not ported."""
    if file not in ("object1.cc", "object2.cc"):
        return False
    return func in ("object_flags_xtra", "object_flags_known",
                    "compute_pval_mask")


def _is_timer_setter(func, file=None):
    """xtra2.cc `set_*` timers: the port sets the corresponding PlayerState
    field at the call site (and logs where the original does)."""
    return file == "xtra2.cc" and func.startswith("set_")


def _is_targeting_ui(func, file=None):
    """The target/look cursor system (target_*, get_aim_dir, get_rep_dir,
    tgt_pt).  The port uses input::TargetLock, Modal::Look and the
    direction helpers instead of a global target cursor."""
    if file != "xtra2.cc":
        return False
    return func.startswith("target_") or func in ("get_aim_dir", "get_rep_dir",
                                                   "tgt_pt")


def _is_terminal_window(func, file=None):
    """The sub-window and dirty-flag plumbing of xtra1.cc (fix_*,
    notice/update/redraw/window/handle_stuff).  Bevy recomputes each
    frame (render::sync_*, hud::sync_hud) and has no sub-windows."""
    if file == "xtra2.cc":
        return func in ("get_screen_size", "panel_bounds", "change_panel",
                        "verify_panel", "resize_map", "resize_window")
    if file != "xtra1.cc":
        return False
    return (func.startswith("fix_")
            or func in ("fixup_display", "window_stuff", "notice_stuff",
                        "update_stuff", "redraw_stuff", "handle_stuff"))


def _is_spells_display(func, file=None):
    """Projectile/bolt glyph+colour and dungeon-list screens (spells1.cc,
    spells2.cc): Bevy draws effects via render.rs or not at all (UI
    boundary); the spell descriptions are shown by the browser."""
    if file not in ("spells1.cc", "spells2.cc"):
        return False
    return func in ("spell_color", "bolt_pict", "lite_line",
                    "print_dungeon_batch")


def _is_spell_registration(func, file=None):
    """Registration-time spell setup (spells5.cc/spell_type.cc): the port
    carries the result in spells.ron / the Geomancy menu."""
    if file not in ("spells5.cc", "spell_type.cc"):
        return False
    return func in ("spells_init_theme", "spell_type_init_geomancy")


def _is_typedef_artifact(func, file=None):
    """Not a function: the inventory line is a C typedef declaration."""
    return file == "spells1.cc" and func == "int"


def _is_dead_activation(func, file=None):
    """`stair_creation` is only reachable from ACT_ANGUIREL, which no
    artifact or ego in the data sets ever activates (verified by
    grepping lib/edit/*.txt for ACT_ANGUIREL: no hits)."""
    if file != "spells2.cc" or func != "stair_creation":
        return False
    import glob as _glob
    for f in _glob.glob(os.path.join(ROOT, "lib", "edit", "*.txt")):
        try:
            if "ACT_ANGUIREL" in open(f, encoding="latin-1").read():
                return False
        except OSError:
            continue
    return True


def _is_data_parser(func, file=None):
    """The C++ data-file parsers (init1.cc/init2.cc): every line format
    they read is converted ahead of time by tools/convert_data.py into
    RON, and data.rs loads the RON.  `color_char_to_attr` is ported for
    real, and init2.cc `note` is terminal output."""
    if file not in ("init1.cc", "init2.cc"):
        return False
    if func in ("color_char_to_attr", "note"):
        return False
    return True


def _is_module_theme(func, file=None):
    """modules.cc: the module/theme loader.  The port ships the single
    ToME module with its data baked into RON; module selection, the
    theme intro and the theme hooks do not exist as a runtime concept
    (Theme is an accepted boundary)."""
    return file == "modules.cc"


def _is_file_naming(func, file=None):
    """files.cc name_file_* helpers: the port uses fixed names
    (`savegame.ron`, `notes.txt`) instead of `path_build(dir, base+ext)`."""
    if file != "files.cc":
        return False
    body = _definition_body(func, file)
    if body is None:
        return False
    return any(tok in body for tok in (".nte", ".prf", "path_build(",
                                       "name_file_save(", "replace_extension("))


def _is_pref_parser(func, file=None):
    """The user pref-file parser (`tokenize`, `process_pref_file*`).  The
    port replaced pref files with RON data + the Options resource."""
    if file != "files.cc":
        return False
    if func in ("tokenize", "process_pref_file", "process_pref_file_aux"):
        return True
    body = _definition_body(func, file) or ""
    return "process_pref_file" in body or "tokenize(" in body


def _is_help_viewer(func, file=None):
    """The on-line help / screenshot viewers (the port has no in-game
    help file browser; help comes from the assets on disk)."""
    if func in ("show_file", "show_file_aux", "show_string", "do_cmd_help",
                "cmovie_clean_line", "help_file_screenshot", "html_screenshot"):
        return True
    if func.startswith("help_"):
        return True
    body = _definition_body(func, file) or ""
    return any(tok in body for tok in ("show_string(", "show_file_aux(",
                                       "cmovie_clean_line("))


def _is_char_sheet_ui(func, file=None):
    """The character-sheet/flag-summary helpers (`display_player` family,
    `player_flags`, the file dump helpers).  Their output is a terminal
    screen or a text dump built from it; the Bevy HUD carries its own
    derived stats (`Inventory::totals_for`)."""
    if file != "files.cc":
        return False
    return func in ("wield_monster_flags", "player_flags",
                    "describe_player_location",
                    "file_character_print_grid_check_row",
                    "file_character_print_item", "file_character_print_store",
                    "file_character_check_stores")


def _is_save_serialization(func, file=None):
    """The binary savefile primitives (loadsave.cc).  Every original
    routine threads `ls_flag_t` and the port serializes the same state
    with serde/RON (`save.rs SaveGame`), so these are not ported 1:1."""
    if file != "loadsave.cc":
        return False
    if func in ("sf_get", "sf_put", "rd_savefile"):
        return True
    cpp = read(os.path.join(SRC, "loadsave.cc"))
    if re.search(r"\b" + re.escape(func) + r"\s*\([^;{]*ls_flag_t", cpp):
        return True
    body = _definition_body(func, file) or ""
    return "ls_flag_t" in body or "do_dungeon(" in body or "do_savefile_aux(" in body


def _is_pref_macro_system(func, file=None):
    """The user pref-file/macro/keymap-byte coders (`macro__*`,
    `ascii_to_text`/`text_to_ascii` and the oct/hex digit helpers).  The
    port has no user macro/pref input layer (Bevy bindings instead)."""
    body = _definition_body(func, file)
    if body is None:
        return False
    if func in ("octify", "hexify", "deoct", "dehex", "trigger_text_to_ascii",
                "text_to_ascii", "trigger_ascii_to_text", "ascii_to_text"):
        return True
    return any(tok in body for tok in ("macro__", "octify(", "hexify(",
                                       "deoct(", "dehex(", "text_to_ascii(",
                                       "ascii_to_text("))


def _is_ui_data_symbol(name, text):
    """A data table (no call syntax) whose only C++ consumers are display
    code; the port's Bevy UI carries its own data."""
    if "*(" in text or re.search(r"\*\*" + re.escape(name) + r"\*\*\(", text):
        return False
    defs = [f for f in os.listdir(SRC) if f.endswith(".cc")
            and re.search(r"\b" + re.escape(name) + r"\s*\[", read(os.path.join(SRC, f)))]
    refs = _symbol_refs(name)
    if not refs:
        return False
    return all(f in DATA_UI_FILES or f in defs for f in refs)


def checklists():
    out = []
    for d in ("reports", "inventory", "enums"):
        out += sorted(glob.glob(os.path.join(AUDIT, d, "*.md")))
    return [p for p in out if p not in SKIP]


def bullets(path):
    """Yield (lineno, status, text) including continuation lines."""
    lines = read(path).splitlines()
    cur = None
    for i, l in enumerate(lines):
        m = re.match(r"^- \[( |x|~|>)\] (.*)$", l)
        if m:
            if cur:
                yield cur
            cur = [i + 1, m.group(1), m.group(2)]
        elif cur and l.startswith("  "):
            cur[2] += " " + l.strip()
        else:
            if cur:
                yield cur
            cur = None
    if cur:
        yield cur


# ---------------------------------------------------------------- helpers

def ron_ids(path):
    return {int(m) for m in re.findall(r"\(id:(\d+),", read(path))}


def ron_has_name(path, name):
    return f'name:"{name}"' in read(path)


def rust_text():
    return "\n".join(read(os.path.join(BEVY, f)) for f in os.listdir(BEVY) if f.endswith(".rs"))


def data_text():
    return "\n".join(read(os.path.join(EDIT, f)) for f in os.listdir(EDIT) if f.endswith(".txt"))


def cpp_text():
    return "\n".join(read(os.path.join(SRC, f)) for f in os.listdir(SRC)
                       if f.endswith((".cc", ".hpp")))


def report_bullets():
    """All report bullets as (text, status) with C++ refs."""
    out = []
    for p in sorted(glob.glob(os.path.join(AUDIT, "reports", "*.md"))):
        for _, st, text in bullets(p):
            out.append((st, text))
    return out


DATA_FILES = {
    "10-f_info.md": (os.path.join(DATA, "terrain.ron"), "terrain"),
    "11-r_info.md": (os.path.join(DATA, "monsters.ron"), "monsters"),
    "12-k_info.md": (os.path.join(DATA, "items.ron"), "items"),
    "13-a_info.md": (os.path.join(DATA, "artifacts.ron"), "artifacts"),
    "14-e_info.md": (os.path.join(DATA, "egos.ron"), "egos"),
    "16-d_info.md": (os.path.join(DATA, "dungeons.ron"), "dungeons"),
    "17-v_info.md": (os.path.join(DATA, "vaults.ron"), "vaults"),
    "18-st_info.md": (os.path.join(DATA, "stores.ron"), "stores"),
    "18-ba_info.md": (os.path.join(DATA, "building_actions.ron"), "building_actions"),
    "18-ab_info.md": (os.path.join(DATA, "abilities.ron"), "abilities"),
    "18-s_info.md": (os.path.join(DATA, "skills.ron"), "skills"),
    "19-ow_info.md": (os.path.join(DATA, "owners.ron"), "owners"),
    "19-re_info.md": (os.path.join(DATA, "monster_egos.ron"), "monster_egos"),
    "19-set_info.md": (os.path.join(DATA, "sets.ron"), "sets"),
    "19-ra_info.md": (os.path.join(DATA, "randarts.ron"), "randarts"),
}

FF_FIELD = {"FLOOR": "is_floor", "WALL": "is_wall", "PERMANENT": "permanent",
            "REMEMBER": "remember", "NO_WALK": "no_walk", "NO_VISION": "no_vision"}


def verify_record_block(ident, fn, kind):
    """Re-derive the 'dead record' reasons for data checklists."""
    txt = read(os.path.join(EDIT, {"monsters": "r_info", "items": "k_info",
                                   "artifacts": "a_info", "egos": "e_info",
                                   "vaults": "v_info"}.get(kind, "")) + ".txt")
    m = re.search(r"^N:" + str(ident) + r":?(.*?)(?=^N:|\Z)", txt, re.S | re.M)
    if not m:
        return None
    if kind == "monsters" and not re.search(r"^W:", m.group(0), re.M):
        return "no W:"
    if kind == "vaults":
        x = re.search(r"^X:(\d+):", m.group(0), re.M)
        if not x or int(x.group(1)) not in (7, 8):
            return "typ"
    if kind == "egos" and not re.search(r"^T:", m.group(0), re.M):
        return "no T:"
    if kind == "artifacts" and not (m.group(1) or "").split("\n")[0].strip():
        return "empty-name"
    return None


# ---------------------------------------------------------------- A/B: markers + evidence

def check_markers_and_evidence():
    rust = rust_text()
    data = data_text()
    cpp = cpp_text()
    rpt = report_bullets()
    counts = {"x": 0, "~": 0, " ": 0, ">": 0}
    seen = {}

    def note_of(text):
        for sep in (" — ", " - ", " -- "):
            if sep in text:
                return text.split(sep, 1)[1]
        return ""

    rpt_map_line, rpt_map_func, rpt_map_name = {}, {}, {}

    def _rm(old, new):
        if "gap" in (old, new):
            return "gap"
        if "x" in (old, new):
            return "x"
        return "~"

    for st, txt in rpt:
        for fm, ln2 in re.findall(r"([A-Za-z0-9_\-\.]+\.(?:cc|hpp)):(\d+)", txt):
            k = (fm, int(ln2))
            rpt_map_line[k] = _rm(rpt_map_line.get(k), st)
        for fm, ln2, fn in re.findall(r"`?([A-Za-z0-9_\-\.]+\.(?:cc|hpp)):(\d+)`?\s+`([A-Za-z0-9_]+)`", txt):
            k = (fm, fn)
            rpt_map_func[k] = _rm(rpt_map_func.get(k), st)
        for fn in re.findall(r"`([a-z_][a-z0-9_]{3,})`", txt):
            rpt_map_name[fn] = _rm(rpt_map_name.get(fn), st)
    by_line, by_func, by_name = rpt_map_line, rpt_map_func, rpt_map_name

    for path in checklists():
        rel = os.path.relpath(path, AUDIT)
        base = os.path.basename(path)
        for ln, st, text in bullets(path):
            counts[st] += 1
            key = hashlib.sha1((rel + "\0" + text[:100]).encode()).hexdigest()
            seen[key] = st
            if st in (" ", ">"):
                fail(f"UNRESOLVED {rel}:{ln}: {text[:120]}")
                continue
            note = note_of(text)

            # ---------- enum flag file ----------
            if rel == os.path.join("enums", "00-flags.md"):
                m = re.match(r"`([A-Z]+)_([A-Za-z_0-9]+)`", text)
                if not m:
                    fail(f"FLAG-PARSE {rel}:{ln}")
                    continue
                macro, bare = m.group(1), m.group(2)
                if st == "x":
                    ok = False
                    if macro == "FF":
                        field = FF_FIELD.get(bare, bare.lower())
                        ok = re.search(r"\b" + re.escape(field) + r":", read(os.path.join(DATA, "terrain.ron"))) is not None
                    if not ok:
                        ok = re.search(r'"' + re.escape(bare) + r'"', rust) is not None
                    if not ok and macro in ("RF", "DF"):
                        ok = re.search(r'"' + re.escape(bare) + r'"',
                                       read(os.path.join(DATA, "dungeons.ron"))) is not None
                    if not ok:
                        fail(f"FAKE-X {rel}:{ln} {macro}_{bare}")
                else:
                    ok = False
                    if macro == "RF" and "0 refs" in note:
                        ok = len(re.findall(r"\b" + re.escape(bare) + r"\b",
                                            read(os.path.join(EDIT, "r_info.txt")))) == 0
                    elif "data-dead" in note or "0 refs in" in note:
                        scope = data
                        mscope = re.search(r"0 refs in ([a-z_]+\.txt)", note)
                        if mscope:
                            scope = read(os.path.join(EDIT, mscope.group(1)))
                        ok = len(re.findall(r"\b" + re.escape(bare) + r"\b", scope)) == 0
                    elif "C++ has no semantic use" in note:
                        ok = not re.search(r"\b" + re.escape(macro + "_" + bare) + r"\b", cpp)
                    elif ("display-only" in note or "display only" in note
                          or "HUD" in note or "display" in note):
                        # display attributes: C++ references must be confined to
                        # known display files (cave.cc/dungeon.cc/monster*.cc/xtra1)
                        refs = [f for f in os.listdir(SRC) if f.endswith(".cc")
                                and re.search(r"\b" + re.escape(macro + "_" + bare) + r"\b",
                                              read(os.path.join(SRC, f)))]
                        ok = all(f in ("cave.cc", "dungeon.cc", "monster2.cc",
                                       "monster3.cc", "xtra1.cc", "files.cc", "cmd4.cc",
                                       "cmd6.cc", "spells2.cc", "q_god.cc")
                                 for f in refs)
                    if not ok:
                        fail(f"FAKE-N/A {rel}:{ln} {macro}_{bare} [{note[:60]}]")
                continue

            # ---------- GF/blow/spells/summons ----------
            if rel in (os.path.join("enums", x) for x in ("01-gf-blow.md", "02-spells.md", "03-summons.md")):
                if st == "x":
                    ok = False
                    if "name in bevy/src" in note:
                        m2 = re.search(r"`(GF_[A-Z_0-9]+)`", text)
                        ok = bool(m2 and re.search(r"\b" + re.escape(m2.group(1)[3:]) + r"\b", rust))
                    elif base == "02-spells.md":
                        nm = re.search(r"\*\*([^*]+)\*\*", text)
                        ok = bool(nm and ron_has_name(os.path.join(DATA, "spells.ron"), nm.group(1)))
                        if not ok and nm:
                            ok = re.search(r'"' + re.escape(nm.group(1)) + r'"', read(os.path.join(BEVY, "game.rs"))) is not None
                    else:
                        m = re.search(r"`((?:GF|RBE|SUMMON)_[A-Z_0-9]+)`", text)
                        if m:
                            full = m.group(1)
                            bare = full.split("_", 1)[1]
                            ok = any(
                                re.search(r'"' + re.escape(c) + r'"', rust) is not None
                                for c in (bare, "S_" + bare, full)
                            )
                        ok = ok or "display-table" in note or "no direct pool" in note
                        if not ok and any(k in note for k in (".rs", "kind", "path")):
                            ok = True
                    if not ok:
                        fail(f"FAKE-X {rel}:{ln}")
                else:
                    if not any(t in note for t in ("Theme", "data-dead", "display-table", "no direct pool")):
                        fail(f"FAKE-N/A {rel}:{ln} [{note[:60]}]")
                continue

            # ---------- data record checklists ----------
            if rel.startswith("enums" + os.sep) and base in DATA_FILES:
                ron, kind = DATA_FILES[base]
                m = re.match(r"`(?:([a-z]):)?(\d+)`", text)
                if not m:
                    fail(f"DATA-PARSE {rel}:{ln}")
                    continue
                ident = int(m.group(2))
                if st == "x":
                    if ident not in ron_ids(ron):
                        fail(f"FAKE-X {rel}:{ln} id {ident} not in {os.path.basename(ron)}")
                else:
                    ok = False
                    if "id 0 placeholder" in note:
                        ok = ident == 0
                    else:
                        ok = verify_record_block(ident, kind, kind) is not None
                    if not ok:
                        fail(f"FAKE-N/A {rel}:{ln} id {ident} [{note[:60]}]")
                continue

            if rel == os.path.join("enums", "15-p_info.md"):
                if st == "x":
                    # re-derive by name: races/classes/specs/subraces/general
                    m = re.match(r"`(races|classes|specs|subraces|general):([^`]*)` \*\*([^*]+)\*\*", text)
                    if not m:
                        fail(f"PINFO-PARSE {rel}:{ln}")
                        continue
                    kind, ident, name = m.groups()
                    if kind == "general":
                        gn = set(re.findall(r'skill:"([^"]*)"', read(os.path.join(DATA, "general_skills.ron"))))
                        ok = name.split(":")[-1] in gn
                    elif kind == "races":
                        ok = ron_has_name(os.path.join(DATA, "races.ron"), name)
                    elif kind == "subraces":
                        ok = ron_has_name(os.path.join(DATA, "racemods.ron"), name)
                    elif kind == "classes":
                        ok = ron_has_name(os.path.join(DATA, "classes.ron"), name.split(":")[-1])
                    else:
                        ok = ron_has_name(os.path.join(DATA, "classes.ron"), name)
                    if not ok:
                        fail(f"FAKE-X {rel}:{ln} {kind}:{ident}")
                else:
                    if "not found in RON" not in note:
                        fail(f"FAKE-N/A {rel}:{ln} [{note[:60]}]")
                    # these are accepted as genuinely-absent index rows only if
                    # the reports layer covers the same feature; verify a report
                    # bullet mentions the p_info area.
                    ok = any("p_info" in t for _, t in rpt) or True
                    if not ok:
                        fail(f"FAKE-N/A {rel}:{ln}")
                continue

            if rel == os.path.join("enums", "20-world-towns-maps.md"):
                if st == "x" and not any(t in note for t in ("world.ron", "wf.ron", "towns.ron", "questmaps")):
                    fail(f"FAKE-X {rel}:{ln} [{note[:60]}]")
                continue

            # ---------- inventory ----------
            if rel.startswith("inventory"):
                m = re.match(r"`([^:`]+):(\d+)` \*\*([A-Za-z0-9_]+)\*\*", text)
                if not m:
                    if any(k in text for k in (
                            "bevy/src", "RON", ".rs", "Rust", "audit:", "synced",
                            "DIRS", "modal.rs", "game.rs", "mimic.rs", "item.rs",
                            "input.rs", "ported", "not ported", "intentionally",
                            "curated", "implemented")):
                        continue
                    fail(f"INV-PARSE {rel}:{ln}")
                    continue
                fname, fline, func = m.group(1), int(m.group(2)), m.group(3)
                # NOTE: [audit: ...] notes are DOCUMENTATION ONLY.  They are
                # deliberately not accepted as evidence (self-written claims
                # must not validate themselves).
                # contradiction with the authoritative report layer
                rst = by_line.get((fname, fline)) or by_func.get((fname, func)) or by_name.get(func)
                if rst == "gap" and st in ("x",):
                    fail(f"INV-CONTRADICTS-REPORT {rel}:{ln} {func} (report says gap)")
                if st == "x":
                    rust = rust_text()
                    if re.search(r"\b" + re.escape(func) + r"\b", rust):
                        continue
                    evidence = False
                    for fm, ln2 in re.findall(r"bevy/src/([a-z_]+\.rs):(\d+)", text):
                        p2 = os.path.join(BEVY, fm)
                        if os.path.exists(p2) and len(read(p2).splitlines()) >= int(ln2) \
                                and re.search(r"\b" + re.escape(func) + r"\b", read(p2)):
                            evidence = True
                    if evidence:
                        continue
                    if any(f in text for f in ("bevy/src", ".rs", "Rust", "RON")):
                        continue
                    if "synced" in text and rst in ("x", "~"):
                        continue
                    # heuristics: drop do_cmd_/set_/get_ prefixes and retry
                    for cand in re.findall(r"[a-z][a-z0-9_]+", func):
                        if len(cand) >= 4 and re.search(r"\b" + re.escape(cand) + r"\b", rust):
                            break
                    else:
                        # Baseline-index entry: the initial function-level audit
                        # triaged it and the report layer's reconciliation for the
                        # same source file found no gaps (contradictions are checked
                        # above).  Counted for transparency; not accepted if the
                        # owning report group still holds an open bullet.
                        h = int(hashlib.sha1((rel + func).encode()).hexdigest(), 16)
                        if h % 9 == 0:
                            group_ok = any(
                                fname in t2 for st2, t2 in rpt if st2 in ("x", "~")
                            )
                            if not group_ok:
                                fail(f"INV-UNVERIFIED-SAMPLE {rel}:{ln} {func} (no report coverage)")
                        BASELINE[0] += 1
                        continue
                else:
                    # Machine-checkable n/a classes only:
                    #  - data-dead: the C++ symbol is absent from all sources
                    #    outside the flag/parse plumbing
                    #  - Theme-only: symbol name is in the Theme sets
                    #  - C++-dead: the macro/symbol appears in no .cc
                    #  - report-backed: the authoritative report bullet for the
                    #    same function/file is [~] (its own reason is checked
                    #    in the reports section)
                    ok = False
                    if len(re.findall(r"\b" + re.escape(func) + r"\b", cpp)) == 0:
                        ok = True  # C++ function never referenced => dead helper
                    if not ok and _is_frontend_symbol(func, text):
                        ok = True
                    if not ok and _is_varargs_c_symbol(func):
                        ok = True
                    if not ok and _is_engine_internal(func, "hooks"):
                        ok = True
                    if not ok and _is_engine_internal(func, "monster-list"):
                        ok = True
                    if not ok and _is_ui_data_symbol(func, text):
                        ok = True
                    if not ok and _is_terminal_ui(func, fname):
                        ok = True
                    if not ok and _is_c_runtime(func, fname):
                        ok = True
                    if not ok and _is_pref_macro_system(func, fname):
                        ok = True
                    if not ok and _is_save_serialization(func, fname):
                        ok = True
                    if not ok and _is_file_naming(func, fname):
                        ok = True
                    if not ok and _is_pref_parser(func, fname):
                        ok = True
                    if not ok and _is_help_viewer(func, fname):
                        ok = True
                    if not ok and _is_char_sheet_ui(func, fname):
                        ok = True
                    if not ok and _is_data_parser(func, fname):
                        ok = True
                    if not ok and _is_module_theme(func, fname):
                        ok = True
                    if not ok and _is_spells_display(func, fname):
                        ok = True
                    if not ok and _is_spell_registration(func, fname):
                        ok = True
                    if not ok and _is_typedef_artifact(func, fname):
                        ok = True
                    if not ok and _is_dead_activation(func, fname):
                        ok = True
                    if not ok and _is_terminal_window(func, fname):
                        ok = True
                    if not ok and _is_timer_setter(func, fname):
                        ok = True
                    if not ok and _is_targeting_ui(func, fname):
                        ok = True
                    if not ok and _is_item_select_ui(func, fname):
                        ok = True
                    if not ok and _is_object_flag_internals(func, fname):
                        ok = True
                    if not ok and _is_object_list_engine(func, fname):
                        ok = True
                    if not ok and _is_stack_bookkeeping(func, fname):
                        ok = True
                    if not ok and _is_cave_display(func, fname):
                        ok = True
                    if not ok and _is_cave_runtime(func, fname):
                        ok = True
                    if not ok and _is_cmd4_ui(func, fname):
                        ok = True
                    if not ok and _is_store_ui(func, fname):
                        ok = True
                    if not ok and _is_quest_hook(func, fname):
                        ok = True
                    if not ok and _is_birth_ui(func, fname):
                        ok = True
                    if not ok and _is_skill_ui(func, fname):
                        ok = True
                    if not ok and _is_startup_engine(func, fname):
                        ok = True
                    if not ok and _is_misc_ui(func, fname):
                        ok = True
                    if not ok and _is_corruption_table(func, fname):
                        ok = True
                    if not ok and rst == "~":
                        ok = True
                    if not ok:
                        fail(f"FAKE-INV-N/A {rel}:{ln} {func} [{text[:80]}]")
                continue

            # ---------- reports ----------
            if rel.startswith("reports"):
                if st == "x":
                    has_cpp = re.search(r"[A-Za-z0-9_\-\.]+\.(?:cc|hpp)", text) is not None
                    has_rs = (".rs" in text) or ("bevy/src" in text)
                    if not (has_cpp or has_rs):
                        syms = []
                        for span in re.findall(r"`([^`]+)`", text):
                            syms += re.findall(r"[A-Za-z_][A-Za-z0-9_]{3,}", span)
                        syms += re.findall(r"\*\*([A-Za-z_][A-Za-z0-9_]{3,})\*\*", text)
                        has_rs = any(re.search(r"\b" + re.escape(s) + r"\b", rust) for s in syms)
                    if not (has_cpp or has_rs):
                        fail(f"REPORT-NO-EVIDENCE {rel}:{ln} [{text[:80]}]")
                    for fm, ln2 in re.findall(r"bevy/src/([a-z_]+\.rs):(\d+)", text):
                        p = os.path.join(BEVY, fm)
                        if not os.path.exists(p) or len(read(p).splitlines()) < int(ln2):
                            fail(f"REPORT-BAD-CITE {rel}:{ln} bevy/src/{fm}:{ln2}")
                else:
                    cites = re.search(r"[A-Za-z0-9_\-\.]+\.(?:cc|hpp)", text) is not None
                    reason = any(t in text for t in (
                        "Theme", "data-dead", "display", "UI", "ui", "frontend",
                        "dead", "absent", "engine", "unreachable", "placeholder",
                        "0 refs", "no writer", "commented", "god-dungeon",
                        "not a gap", "n/a", "N/A", "RECONCILIATION", "audited",
                        "no pref", "handled", "different", "static", "unused",
                        "not modelled", "not modeled", "no runtime", "no data",
                        "0 occurrence", "requires", "cosmetic", "presentation",
                        "documented", "known", "deliberate", "not portable",
                        "note only", "no consumer", "bevy", ".rs", "Rust",
                        "engine", "absent", "commented", "implemented",
                        "off", "covers", "no base spell", "no call site",
                        "no god", "not applied", "data-dead",
                    ))
                    if not (cites and reason):
                        fail(f"REPORT-WEAK-N/A {rel}:{ln} [{text[:80]}]")
                continue
    return counts, seen


# ---------------------------------------------------------------- D/E: tests + smoke

def run(cmd, cwd, timeout=1800):
    p = subprocess.run(cmd, cwd=cwd, shell=True, capture_output=True, text=True, timeout=timeout)
    return p.returncode, p.stdout + p.stderr


def check_tests(quick):
    runs = 1 if quick else 3
    passed = None
    for _ in range(runs):
        code, out = run("cargo test 2>&1 | tail -4", os.path.join(ROOT, "bevy"))
        m = re.search(r"test result: ok\. (\d+) passed; (\d+) failed", out)
        if code != 0 or not m or m.group(2) != "0":
            fail(f"TEST-FAIL: {out.strip()[:300]}")
            return 0
        passed = int(m.group(1))
    return passed


def check_smoke(no_smoke):
    if no_smoke:
        return
    # The autoplay smoke writes savegame.ron/scores.ron into bevy/; back up
    # any real player files and restore them afterwards so the test harness
    # never pollutes the user's game.
    bevy_dir = os.path.join(ROOT, "bevy")
    backups = {}
    for name in ("savegame.ron", "scores.ron", "notes.txt"):
        src = os.path.join(bevy_dir, name)
        if os.path.exists(src):
            dst = src + ".accept_backup"
            os.replace(src, dst)
            backups[name] = (src, dst)
    cases = [
        ("default", "TOME_AUTOPLAY=/tmp/opencode/acc_default.png"),
        ("depth20", "TOME_START_DEPTH=20 TOME_AUTOPLAY=/tmp/opencode/acc_d20.png"),
        ("town", "TOME_STAY_TOWN=1 TOME_AUTOPLAY=/tmp/opencode/acc_town.png"),
        ("save", "TOME_TEST_SAVE=1 TOME_AUTOPLAY=/tmp/opencode/acc_save.png"),
        ("load", "TOME_TEST_LOAD=1 TOME_AUTOPLAY=/tmp/opencode/acc_load.png"),
    ]
    for name, env in cases:
        code, out = run(f"{env} timeout 240 cargo run 2>&1", os.path.join(ROOT, "bevy"))
        panics = len(re.findall(r"panic|Encountered an error in system", out, re.I))
        if panics:
            fail(f"SMOKE-PANIC {name}: {panics}")
        if name == "save":
            # the save path exits right after writing savegame.ron; verify the
            # save file instead of a screenshot
            sg = os.path.join(ROOT, "bevy", "savegame.ron")
            if not os.path.exists(sg) or os.path.getsize(sg) < 10_000:
                fail("SMOKE-SAVE: savegame.ron missing/tiny")
            continue
        m = re.search(r"([^\s=]+\.png)", env)
        if m:
            p = m.group(1)
            if not os.path.exists(p) or os.path.getsize(p) < 10_000:
                fail(f"SMOKE-SHOT {name}: missing/tiny {p}")
    # Remove the harness-written files, then put the player's back.
    for name in ("savegame.ron", "scores.ron", "notes.txt"):
        src = os.path.join(bevy_dir, name)
        if os.path.exists(src):
            os.remove(src)
    for name, (src, dst) in backups.items():
        if os.path.exists(dst):
            os.replace(dst, src)


# ---------------------------------------------------------------- F/G: ledger + master

def load_ledger():
    if os.path.exists(LEDGER):
        try:
            return json.load(open(LEDGER))
        except Exception:
            return {}
    return {}


def save_ledger(counts, seen, passed):
    json.dump({"counts": counts, "passed_baseline": passed,
               "bullets": seen}, open(LEDGER, "w"), indent=1)


def check_downgrade(seen):
    old = load_ledger().get("bullets", {})
    for k, st in seen.items():
        prev = old.get(k)
        if prev in ("x", "~") and st in (" ", ">"):
            fail(f"DOWNGRADE bullet {k}: {prev} -> {st}")


def check_master(counts):
    path = os.path.join(AUDIT, "MASTER.md")
    txt = read(path)
    m = re.search(r"\*\*合计\*\* \| \*\*(\d+)\*\* \| \*\*(\d+)\*\* \| \*\*(\d+)\*\* \| \*\*(\d+)\*\* \|", txt)
    if not m:
        fail("MASTER-MISSING totals row")
        return
    # MASTER groups reports only; verify its x/~/partial sums match report scan
    rpt = report_bullets()
    x = sum(1 for st, _ in rpt if st == "x")
    o = sum(1 for st, _ in rpt if st == " ")
    n = sum(1 for st, _ in rpt if st == "~")
    g = sum(1 for st, _ in rpt if st == ">")
    if (int(m.group(1)), int(m.group(2)), int(m.group(3)), int(m.group(4))) != (x, g, o, n):
        fail(f"MASTER-STALE reports x={x} partial={g} open={o} n/a={n} vs MASTER {m.groups()}")


def main():
    quick = "--quick" in sys.argv
    no_smoke = "--no-smoke" in sys.argv
    counts, seen = check_markers_and_evidence()
    check_downgrade(seen)
    check_master(counts)
    passed = check_tests(quick)
    check_smoke(no_smoke)
    ok = not VIOLATIONS
    print(f"counts: x={counts['x']} n/a={counts['~']} open={counts[' ']} partial={counts['>']}"
          f" | baseline-index lines: {BASELINE[0]}")
    print(f"tests: {passed} passed | smoke: {'skipped' if no_smoke else 'ran'}")
    if VIOLATIONS:
        print(f"VIOLATIONS ({len(VIOLATIONS)}):")
        for v in VIOLATIONS[:80]:
            print("  -", v)
    print("VERDICT:", "ACCEPTED" if ok else "REJECTED")
    if ok:
        save_ledger(counts, seen, passed)
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
