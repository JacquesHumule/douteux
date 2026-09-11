use std::str::FromStr;

use serde::Deserialize;

use crate::generators::rng::Rng;

pub mod crack;
pub mod movie;
pub mod pup;
mod rng;
pub mod tape;

// ---------------------------------------------------------------------------
// Slug generation
// ---------------------------------------------------------------------------

const DEFAULT_SLUG_LEN: usize = 6;
const MIN_SLUG_LEN: usize = 3;
const MAX_SLUG_LEN: usize = 64;

/// Separates a pattern's mode from its argument, e.g. `shorten:12`.
const ARG_DELIM: char = ':';

const SLUG_ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

/// Which slug shape to generate.
///
/// Deserialized (by serde, from the `pattern` JSON field or query param) as a
/// `mode` string with an optional `:arg` suffix:
///
/// | input         | result                                        |
/// |---------------|-----------------------------------------------|
/// | `shorten`     | random slug, [`DEFAULT_SLUG_LEN`] chars        |
/// | `shorten:12`  | random slug, 12 chars (clamped to 3..=64)      |
/// | `shady`       | disguised slug, default kind                   |
/// | `shady:movie` | fake torrent-release-style name                |
/// | `shady:crack` | fake software-crack release name              |
/// | `shady:pup`   | fake "PC optimizer" / freeware-installer name  |
/// | `shady:tape`  | weird / disturbing "recovered video" name      |
/// | empty         | [`SlugPattern::default`] — `shady`             |
#[derive(Debug, Deserialize)]
#[serde(try_from = "String")]
pub enum SlugPattern {
    /// Random alphanumeric slug of the given length.
    Shorten { len: usize },
    /// A slug disguised as something else.
    Shady(ShadyPattern),
}

impl Default for SlugPattern {
    fn default() -> Self {
        SlugPattern::Shady(ShadyPattern::default())
    }
}

impl FromStr for SlugPattern {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (mode, arg) = match s.split_once(ARG_DELIM) {
            Some((mode, arg)) => (mode, Some(arg)),
            None => (s, None),
        };

        match mode {
            "" => Ok(SlugPattern::default()),
            "shorten" => {
                let len = match arg {
                    Some(arg) => arg
                        .parse::<usize>()
                        .map_err(|_| format!("invalid slug length {arg:?}"))?
                        .clamp(MIN_SLUG_LEN, MAX_SLUG_LEN),
                    None => DEFAULT_SLUG_LEN,
                };
                Ok(SlugPattern::Shorten { len })
            }
            "shady" => Ok(SlugPattern::Shady(
                arg.map(ShadyPattern::from_str)
                    .transpose()?
                    .unwrap_or_default(),
            )),
            other => Err(format!("unknown pattern {other:?}")),
        }
    }
}

impl TryFrom<String> for SlugPattern {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.parse()
    }
}

/// The disguise used by [`SlugPattern::Shady`].
#[derive(Debug, Default)]
pub enum ShadyPattern {
    /// Fake torrent-release-style name, e.g.
    /// `The.Matrix.1999.VOSTFR.1080p.BluRay.DDP5.1.x264-NOVA[A3F9C1D2].mp4`.
    #[default]
    Movie,
    /// Fake software-crack release name, e.g.
    /// `Adobe.Photoshop.v25.3.1.Multilingual.Incl.Keygen-KEYFORGE[A3F9C1D2].rar`.
    Crack,
    /// Fake "PC optimizer" / freeware-installer name, e.g.
    /// `SmartDriverBooster_Pro_Setup_v8.4_x64[A3F9C1D2].exe`.
    Pup,
    /// Weird / disturbing "recovered video" name, e.g.
    /// `DO_NOT_WATCH_the_smiling_woman_attic_3am_no_audio_tape_07[A3F9C1D2].avi`.
    Tape,
}

impl FromStr for ShadyPattern {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "movie" => Ok(ShadyPattern::Movie),
            "crack" => Ok(ShadyPattern::Crack),
            "pup" => Ok(ShadyPattern::Pup),
            "tape" => Ok(ShadyPattern::Tape),
            other => Err(format!("unknown shady kind {other:?}")),
        }
    }
}

pub fn generate_slug(pattern: SlugPattern) -> String {
    match pattern {
        SlugPattern::Shorten { len } => random_slug(len),
        SlugPattern::Shady(ShadyPattern::Movie) => movie::generate(),
        SlugPattern::Shady(ShadyPattern::Crack) => crack::generate(),
        SlugPattern::Shady(ShadyPattern::Pup) => pup::generate(),
        SlugPattern::Shady(ShadyPattern::Tape) => tape::generate(),
    }
}

fn random_slug(len: usize) -> String {
    let mut rng = Rng::new();
    (0..len).map(|_| *rng.pick(SLUG_ALPHABET) as char).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> Result<SlugPattern, String> {
        s.parse()
    }

    #[test]
    fn empty_input_is_default() {
        assert!(matches!(
            parse(""),
            Ok(SlugPattern::Shady(ShadyPattern::Movie))
        ));
        assert!(matches!(
            SlugPattern::default(),
            SlugPattern::Shady(ShadyPattern::Movie)
        ));
    }

    #[test]
    fn shorten_without_arg_uses_default_len() {
        assert!(matches!(
            parse("shorten"),
            Ok(SlugPattern::Shorten { len }) if len == DEFAULT_SLUG_LEN
        ));
    }

    #[test]
    fn shorten_with_explicit_len() {
        assert!(matches!(
            parse("shorten:12"),
            Ok(SlugPattern::Shorten { len: 12 })
        ));
    }

    #[test]
    fn shorten_len_is_clamped_to_bounds() {
        assert!(matches!(
            parse("shorten:0"),
            Ok(SlugPattern::Shorten { len }) if len == MIN_SLUG_LEN
        ));
        assert!(matches!(
            parse("shorten:1000"),
            Ok(SlugPattern::Shorten { len }) if len == MAX_SLUG_LEN
        ));
    }

    #[test]
    fn shorten_with_invalid_len_errors() {
        assert!(parse("shorten:abc").is_err());
        assert!(parse("shorten:-1").is_err());
        assert!(parse("shorten:1.5").is_err());
        assert!(parse("shorten:").is_err()); // trailing delimiter, empty arg
    }

    #[test]
    fn shady_without_arg_is_default_kind() {
        assert!(matches!(
            parse("shady"),
            Ok(SlugPattern::Shady(ShadyPattern::Movie))
        ));
    }

    #[test]
    fn shady_movie() {
        assert!(matches!(
            parse("shady:movie"),
            Ok(SlugPattern::Shady(ShadyPattern::Movie))
        ));
    }

    #[test]
    fn shady_crack() {
        assert!(matches!(
            parse("shady:crack"),
            Ok(SlugPattern::Shady(ShadyPattern::Crack))
        ));
    }

    #[test]
    fn shady_pup() {
        assert!(matches!(
            parse("shady:pup"),
            Ok(SlugPattern::Shady(ShadyPattern::Pup))
        ));
    }

    #[test]
    fn shady_tape() {
        assert!(matches!(
            parse("shady:tape"),
            Ok(SlugPattern::Shady(ShadyPattern::Tape))
        ));
    }

    #[test]
    fn shady_unknown_kind_errors() {
        assert!(parse("shady:crypto").is_err());
        assert!(parse("shady:").is_err());
    }

    #[test]
    fn unknown_mode_errors() {
        assert!(parse("movie").is_err());
        assert!(parse("garbage").is_err());
    }

    #[test]
    fn parsing_is_case_sensitive() {
        assert!(parse("Shorten").is_err());
        assert!(parse("SHADY:MOVIE").is_err());
    }

    #[test]
    fn only_the_first_delimiter_splits_the_arg() {
        // "12:34" is passed whole to the length parser, which rejects it.
        assert!(parse("shorten:12:34").is_err());
    }

    #[test]
    fn deserializes_from_a_json_string() {
        let p: SlugPattern = serde_json::from_str(r#""shorten:8""#).unwrap();
        assert!(matches!(p, SlugPattern::Shorten { len: 8 }));

        let p: SlugPattern = serde_json::from_str(r#""shady:movie""#).unwrap();
        assert!(matches!(p, SlugPattern::Shady(ShadyPattern::Movie)));

        let p: SlugPattern = serde_json::from_str(r#""shady:crack""#).unwrap();
        assert!(matches!(p, SlugPattern::Shady(ShadyPattern::Crack)));

        let p: SlugPattern = serde_json::from_str(r#""shady:pup""#).unwrap();
        assert!(matches!(p, SlugPattern::Shady(ShadyPattern::Pup)));

        let p: SlugPattern = serde_json::from_str(r#""shady:tape""#).unwrap();
        assert!(matches!(p, SlugPattern::Shady(ShadyPattern::Tape)));
    }

    #[test]
    fn deserializing_an_invalid_pattern_fails() {
        assert!(serde_json::from_str::<SlugPattern>(r#""nope""#).is_err());
        assert!(serde_json::from_str::<SlugPattern>(r#""shorten:x""#).is_err());
    }

    #[test]
    fn optional_pattern_field_defaults_when_absent() {
        #[derive(serde::Deserialize)]
        struct Wrap {
            #[serde(default)]
            pattern: Option<SlugPattern>,
        }

        let w: Wrap = serde_json::from_str("{}").unwrap();
        assert!(w.pattern.is_none());

        let w: Wrap = serde_json::from_str(r#"{"pattern":"shorten:20"}"#).unwrap();
        assert!(matches!(
            w.pattern.unwrap_or_default(),
            SlugPattern::Shorten { len: 20 }
        ));
    }
}
