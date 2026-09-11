//! Generates weird / disturbing "recovered video" filenames, e.g.
//! `DO_NOT_WATCH_the_smiling_woman_attic_3am_no_audio_tape_07[A3F9C1D2].avi`
//!
//! The vibe is analog-horror / found-footage / cursed-VHS: the kind of name an
//! unlisted video or a "please help I don't know what this is" upload would have.
//! Every phrase here is fictional and generic — no real people, places or events.

use crate::generators::rng::Rng;

/// Optional leading tag — the "you shouldn't be seeing this" note.
const WARNINGS: &[&str] = &[
    "DO_NOT_WATCH",
    "do_not_share",
    "PLEASE_HELP",
    "nsfl",
    "unlisted",
    "RECOVERED",
    "found_footage",
    "deleted",
    "real",
    "evidence",
    "last_upload",
    "for_my_family",
    "does_anyone_know_what_this_is",
    "turn_your_volume_down",
];

/// The core "what's on the tape" phrase. Always present.
const SUBJECTS: &[&str] = &[
    "the_man_in_the_hallway",
    "she_keeps_knocking",
    "dad_wont_wake_up",
    "something_under_the_stairs",
    "my_neighbor_at_night",
    "the_thing_that_hums",
    "grandmas_house",
    "the_smiling_woman",
    "room_237",
    "the_static_family",
    "whatever_is_in_the_vents",
    "the_kids_from_the_field",
    "it_was_at_the_window_again",
    "the_long_hallway",
    "mom_in_the_yard",
    "the_visitor_wont_leave",
    "he_is_still_downstairs",
    "the_second_shadow",
    "why_is_it_counting",
    "the_birthday_party",
];

/// Optional location.
const PLACES: &[&str] = &[
    "basement",
    "attic",
    "crawlspace",
    "back_bedroom",
    "parking_garage",
    "rest_stop_bathroom",
    "the_woods_behind_the_school",
    "apartment_4b",
    "storage_unit_112",
    "the_lake_house",
    "end_of_the_cul_de_sac",
    "motel_room_9",
];

/// Optional time / date stamp.
const TIMES: &[&str] = &[
    "3am",
    "4_17am",
    "after_midnight",
    "august_1997",
    "christmas_1994",
    "night_2_of_3",
    "day_47",
    "same_night",
    "before_the_power_cut",
];

/// Optional camera / tape condition note.
const CAMERA: &[&str] = &[
    "no_audio",
    "night_vision",
    "handheld",
    "motion_activated",
    "corrupted",
    "last_5_minutes",
    "reversed",
    "slowed_down",
    "audio_only",
    "cam03",
    "dashcam",
    "baby_monitor",
    "unedited",
    "do_not_adjust_the_tracking",
];

/// Label for the "tape 07" style reel token.
const TAPE_LABELS: &[&str] = &["tape", "reel", "disc", "clip", "vhs"];

/// Dated / lo-fi video container extensions.
const EXTENSIONS: &[&str] = &[
    "avi", "wmv", "mov", "mpg", "mp4", "mkv", "vob", "3gp", "flv", "rm",
];

/// Generate a weird / disturbing "recovered video" name.
pub fn generate() -> String {
    let mut rng = Rng::new();
    let mut parts: Vec<String> = Vec::new();

    // Mostly lead with a "you shouldn't be watching this" tag.
    if rng.next_u64() % 100 < 55 {
        parts.push((*rng.pick(WARNINGS)).to_string());
    }

    // The one part that's always there.
    parts.push((*rng.pick(SUBJECTS)).to_string());

    if rng.next_u64() % 100 < 40 {
        parts.push((*rng.pick(PLACES)).to_string());
    }
    if rng.next_u64() % 100 < 45 {
        parts.push((*rng.pick(TIMES)).to_string());
    }
    if rng.next_u64() % 100 < 55 {
        parts.push((*rng.pick(CAMERA)).to_string());
    }
    // A "tape 07" style reel token.
    if rng.next_u64() % 100 < 50 {
        let label = rng.pick(TAPE_LABELS);
        let n = 1 + rng.next_u64() % 42;
        parts.push(format!("{label}_{n:02}"));
    }

    let ext = rng.pick(EXTENSIONS);
    // Build-tag — also keeps slugs from colliding.
    let hash = rng.hex(8);

    format!("{}[{hash}].{ext}", parts.join("_"))
}
