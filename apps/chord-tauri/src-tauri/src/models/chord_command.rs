use super::{ChordLayout, ChordMacroName, ChordMacros, Key, expand_typed_input};
use anyhow::{Context, Result};
use keycode::KeyMappingCode;
use std::time::Duration;

pub const CORE_CHORD_PACKAGE: &str = "@keychord/chords-chord";
pub const CORE_CHORD_REPO: &str = "KeyChord/chords-chord";
pub const CORE_CHORD_FILE: &str = "chords/com/leonsilicon/chord/macos.toml";

pub fn is_core_chords_file(package: &str, path: &std::path::Path) -> bool {
    package == CORE_CHORD_PACKAGE && path == std::path::Path::new(CORE_CHORD_FILE)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetaChordCommand {
    Settings,
    Reload,
    Quit,
    Halt(u32),
    SetLayout(ChordLayout),
    CycleLayout,
    SetMacro {
        name: ChordMacroName,
        sequence: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChordRepeat {
    Times(u32),
    Loop,
}

/// Options written between backticks and separated by commas, after a repeat count
/// (`` 3`d2,i13`ab ``), at the start of the input (`` `d5`ab ``), or after a quote
/// (`` 'hello'`i5` ``).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChordRunConfig {
    /// Pause before the first run, once; loops and repeats never wait for it again.
    pub delay: Option<Duration>,
    /// Pauses between consecutive chord runs or typed keys, used in order and cycled.
    pub intervals: Vec<Duration>,
}

impl ChordRunConfig {
    /// Digits count hundreds of milliseconds: `d` takes exactly one, and `i` takes one
    /// per interval, where `0` runs the next chord immediately. Each option appears at
    /// most once. Unknown or malformed options are rejected.
    fn parse(keys: &[Key]) -> Option<Self> {
        let source = Key::serialize_sequence(keys)?;
        let mut config = Self::default();
        if source.is_empty() {
            return Some(config);
        }
        for option in source.split(',') {
            let mut chars = option.chars();
            let name = chars.next()?;
            let pauses = chars
                .map(|digit| {
                    let tenths = u64::from(digit.to_digit(10)?);
                    Some(Duration::from_millis(tenths * 100))
                })
                .collect::<Option<Vec<_>>>()?;
            match (name, pauses.as_slice()) {
                ('d', &[delay]) if config.delay.is_none() => config.delay = Some(delay),
                ('i', [_, ..]) if config.intervals.is_empty() => config.intervals = pauses,
                _ => return None,
            }
        }
        Some(config)
    }
}

/// Splits off a leading `` `config` `` block. `None` while it is unclosed or invalid.
fn split_config(keys: &[Key]) -> Option<(Option<ChordRunConfig>, &[Key])> {
    if keys.first() != Some(&Key(KeyMappingCode::Backquote)) {
        return Some((None, keys));
    }
    let length = keys[1..]
        .iter()
        .position(|key| key.0 == KeyMappingCode::Backquote)?;
    let config = ChordRunConfig::parse(&keys[1..=length])?;
    Some((Some(config), &keys[length + 2..]))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChordStep {
    Chord(Vec<Key>),
    /// Types `text` into the focused app. A config block after the closing quote sets
    /// intervals between characters; without one they are typed immediately.
    Quote {
        text: String,
        intervals: Vec<Duration>,
    },
}

enum Unfinished {
    /// The input ends with an open quote; `quote` indexes its insert-mode text.
    Open {
        quote: usize,
    },
    Pending,
}

/// Parses one step, or Enter-separated steps when `separated`. A quote typed in
/// insert mode is an adjacent `''` pair whose text is the next entry of `quotes`;
/// text notation (TOML, CLI) writes unshifted keys between the quotes instead.
fn parse_steps(
    mut body: &[Key],
    separated: bool,
    quotes: &[String],
) -> Result<Vec<ChordStep>, Unfinished> {
    let mut steps = Vec::new();
    let mut quote = 0;
    loop {
        let rest = if body.first() == Some(&Key(KeyMappingCode::Quote)) {
            // Only `'` closes a quote; its text can contain Enter and backticks.
            let Some(length) = body[1..]
                .iter()
                .position(|key| key.0 == KeyMappingCode::Quote)
            else {
                return Err(if body.len() == 1 {
                    Unfinished::Open { quote }
                } else {
                    Unfinished::Pending
                });
            };
            let content = &body[1..=length];
            let text = if content.is_empty() {
                quote += 1;
                quotes.get(quote - 1).cloned().unwrap_or_default()
            } else {
                content
                    .iter()
                    .map(|key| key.text_char(false))
                    .collect::<Option<String>>()
                    .ok_or(Unfinished::Pending)?
            };
            let (config, rest) = split_config(&body[length + 2..]).ok_or(Unfinished::Pending)?;
            let config = config.unwrap_or_default();
            // A delay belongs to the whole run, so a step cannot wait again each loop.
            if text.is_empty() || config.delay.is_some() {
                return Err(Unfinished::Pending);
            }
            steps.push(ChordStep::Quote {
                text,
                intervals: config.intervals,
            });
            rest
        } else {
            let end = body
                .iter()
                .position(|key| separated && key.0 == KeyMappingCode::Enter)
                .unwrap_or(body.len());
            // Zero commands cannot be sequenced, and an empty chord is still being typed.
            if body[..end]
                .first()
                .is_none_or(|key| key.0 == KeyMappingCode::Digit0)
            {
                return Err(Unfinished::Pending);
            }
            steps.push(ChordStep::Chord(body[..end].to_vec()));
            &body[end..]
        };
        match rest.split_first() {
            None => return Ok(steps),
            Some((key, next)) if separated && key.0 == KeyMappingCode::Enter => body = next,
            Some(_) => return Err(Unfinished::Pending),
        }
    }
}

/// Text notation for typed input: each insert-mode quote's `''` pair shows its text.
pub fn serialize_with_quotes(input: &[Key], quotes: &[String]) -> Option<String> {
    let mut quotes = quotes.iter();
    let mut output = String::new();
    let mut keys = input.iter().peekable();
    while let Some(key) = keys.next() {
        if key.0 == KeyMappingCode::Quote
            && keys
                .peek()
                .is_some_and(|next| next.0 == KeyMappingCode::Quote)
        {
            if let Some(text) = quotes.next() {
                keys.next();
                output.push('\'');
                output.push_str(text);
                output.push('\'');
                continue;
            }
        }
        output.push_str(&Key::serialize_sequence(std::slice::from_ref(key))?);
    }
    Some(output)
}

/// The number of insert-mode quotes in typed `input`, counted like
/// `serialize_with_quotes`: one per adjacent `''` pair.
pub fn typed_quote_count(input: &[Key]) -> usize {
    let mut count = 0;
    let mut keys = input.iter().peekable();
    while let Some(key) = keys.next() {
        if key.0 == KeyMappingCode::Quote
            && keys
                .next_if(|next| next.0 == KeyMappingCode::Quote)
                .is_some()
        {
            count += 1;
        }
    }
    count
}

#[derive(Debug, PartialEq, Eq)]
pub enum ChordCommand {
    Pending,
    /// The input ends with an open quote, so keys are typed into the text at index
    /// `quote` of the insert-mode quotes until Caps+Space closes it.
    Insert {
        quote: usize,
    },
    /// All input starting with zero belongs to Chord, including unknown commands.
    Meta {
        keys: Vec<Key>,
    },
    Run {
        /// Never empty. Enter separates steps only after a repeat count or config block.
        steps: Vec<ChordStep>,
        repeat: ChordRepeat,
        config: ChordRunConfig,
    },
}

impl ChordCommand {
    /// `quotes` holds the insert-mode text of each quote in `input`, in order.
    pub fn parse(input: &[Key], quotes: &[String], macros: &ChordMacros) -> Result<Self> {
        // A backslash introduces a buffer reference or an escape, never a literal key.
        // Unselected escapes stay editable without dispatching handlers.
        let Some(input) = expand_typed_input(input, macros)? else {
            return Ok(Self::Pending);
        };
        if input.first() == Some(&Key(KeyMappingCode::Digit0)) {
            return Ok(Self::Meta { keys: input });
        }
        let prefix_length = input.iter().take_while(|key| key.is_digit()).count();
        let repeat = if prefix_length == 0 {
            ChordRepeat::Times(1)
        } else {
            let count = Key::serialize_sequence(&input[..prefix_length])
                .context("invalid repeat prefix")?
                .parse::<u32>()
                .context("repeat count must fit in an unsigned 32-bit integer")?;
            if count == 1 {
                ChordRepeat::Loop
            } else {
                ChordRepeat::Times(count)
            }
        };
        let Some((config, body)) = split_config(&input[prefix_length..]) else {
            return Ok(Self::Pending);
        };
        // The run syntax splits consecutive steps at Enter; elsewhere Enter is a key.
        match parse_steps(body, prefix_length > 0 || config.is_some(), quotes) {
            Ok(steps) => Ok(Self::Run {
                steps,
                repeat,
                config: config.unwrap_or_default(),
            }),
            Err(Unfinished::Open { quote }) => Ok(Self::Insert { quote }),
            Err(Unfinished::Pending) => Ok(Self::Pending),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Text notation stands for already-selected keys, e.g. `\n` is the Enter key.
    fn parse(input: &str) -> Result<ChordCommand> {
        ChordCommand::parse(&Key::parse_sequence(input)?, &[], &ChordMacros::default())
    }

    fn parse_keys(input: &[Key]) -> ChordCommand {
        ChordCommand::parse(input, &[], &ChordMacros::default()).unwrap()
    }

    fn parse_typed(input: &str, quotes: &[&str]) -> ChordCommand {
        let quotes: Vec<String> = quotes.iter().map(|quote| (*quote).into()).collect();
        ChordCommand::parse(
            &Key::parse_sequence(input).unwrap(),
            &quotes,
            &ChordMacros::default(),
        )
        .unwrap()
    }

    fn millis(delays: &[u64]) -> Vec<Duration> {
        delays.iter().copied().map(Duration::from_millis).collect()
    }

    fn chord(keys: &str) -> ChordStep {
        ChordStep::Chord(Key::parse_sequence(keys).unwrap())
    }

    fn quote(text: &str, intervals: &[u64]) -> ChordStep {
        ChordStep::Quote {
            text: text.into(),
            intervals: millis(intervals),
        }
    }

    fn steps(steps: Vec<ChordStep>, repeat: ChordRepeat, intervals: &[u64]) -> ChordCommand {
        ChordCommand::Run {
            steps,
            repeat,
            config: ChordRunConfig {
                delay: None,
                intervals: millis(intervals),
            },
        }
    }

    fn run(chords: &[&str], repeat: ChordRepeat, intervals: &[u64]) -> ChordCommand {
        steps(
            chords.iter().map(|keys| chord(keys)).collect(),
            repeat,
            intervals,
        )
    }

    fn delayed(mut command: ChordCommand, delay: u64) -> ChordCommand {
        if let ChordCommand::Run { config, .. } = &mut command {
            config.delay = Some(Duration::from_millis(delay));
        }
        command
    }

    #[test]
    fn numeric_prefixes_do_not_consume_numeric_arguments() {
        for (input, command, repeat) in [
            ("hsr12", "hsr12", ChordRepeat::Times(1)),
            ("12hsr34", "hsr34", ChordRepeat::Times(12)),
            ("10ab", "ab", ChordRepeat::Times(10)),
            ("1ab", "ab", ChordRepeat::Loop),
        ] {
            assert_eq!(parse(input).unwrap(), run(&[command], repeat, &[]));
        }
        assert_eq!(parse("12").unwrap(), ChordCommand::Pending);
        assert!(parse("4294967296ab").is_err());
    }

    #[test]
    fn zero_always_selects_the_reserved_namespace() {
        for input in ["0s", "0r", "0q", "0h12", "0", "0h", "0h12x", "00ab"] {
            assert_eq!(
                parse(input).unwrap(),
                ChordCommand::Meta {
                    keys: Key::parse_sequence(input).unwrap(),
                }
            );
        }
    }

    #[test]
    fn typed_escapes_stay_pending_until_tab_selects_an_expansion() {
        for (word, key) in [
            ("up", KeyMappingCode::ArrowUp),
            ("left", KeyMappingCode::ArrowLeft),
            ("down", KeyMappingCode::ArrowDown),
            ("right", KeyMappingCode::ArrowRight),
            ("n", KeyMappingCode::Enter),
        ] {
            // Without a count, so Enter stays inside the chord.
            let mut input = Key::parse_sequence("a").unwrap();
            input.push(Key(KeyMappingCode::Backslash));
            assert_eq!(parse_keys(&input), ChordCommand::Pending);
            for letter in word.chars() {
                input.push(letter.to_string().parse().unwrap());
                // Even a complete escape never expands on its own.
                assert_eq!(parse_keys(&input), ChordCommand::Pending);
            }
            let expected = ChordCommand::Run {
                steps: vec![ChordStep::Chord(vec![Key(KeyMappingCode::KeyA), Key(key)])],
                repeat: ChordRepeat::Times(1),
                config: ChordRunConfig::default(),
            };
            let selected = crate::models::select_escape(&input, &ChordMacros::default())
                .unwrap()
                .unwrap();
            assert_eq!(parse_keys(&selected), expected);
            let mut physical = Key::parse_sequence("a").unwrap();
            physical.push(Key(key));
            assert_eq!(parse_keys(&physical), expected);
        }
        // Former system-action chords and unknown escapes never reach a handler.
        for prefix in ["", "0", "12a"] {
            for suffix in ["", "s", "q", "r", "unknown"] {
                let mut input = Key::parse_sequence(prefix).unwrap();
                input.push(Key(KeyMappingCode::Backslash));
                input.extend(Key::parse_sequence(suffix).unwrap());
                assert_eq!(parse_keys(&input), ChordCommand::Pending);
            }
        }
    }

    #[test]
    fn backtick_config_sets_a_delay_and_intervals_between_runs() {
        for (input, expected) in [
            ("3`i5`ab", run(&["ab"], ChordRepeat::Times(3), &[500])),
            ("1`i13`ab", run(&["ab"], ChordRepeat::Loop, &[100, 300])),
            ("`i9`ab", run(&["ab"], ChordRepeat::Times(1), &[900])),
            ("2`i0`ab", run(&["ab"], ChordRepeat::Times(2), &[0])),
            ("2`i10`ab", run(&["ab"], ChordRepeat::Times(2), &[100, 0])),
            ("2``ab", run(&["ab"], ChordRepeat::Times(2), &[])),
            // `d` delays any chord once, with or without a count, before its first run.
            (
                "`d5`ab",
                delayed(run(&["ab"], ChordRepeat::Times(1), &[]), 500),
            ),
            (
                "`d0`ab",
                delayed(run(&["ab"], ChordRepeat::Times(1), &[]), 0),
            ),
            (
                "3`d2,i130`/ws]",
                delayed(run(&["/ws]"], ChordRepeat::Times(3), &[100, 300, 0]), 200),
            ),
            (
                "1`i4,d9`ab",
                delayed(run(&["ab"], ChordRepeat::Loop, &[400]), 900),
            ),
            // Only a leading block is config; later backticks are ordinary keys.
            ("2gb`", run(&["gb`"], ChordRepeat::Times(2), &[])),
            ("2`i5`12ab", run(&["12ab"], ChordRepeat::Times(2), &[500])),
        ] {
            assert_eq!(parse(input).unwrap(), expected, "{input}");
        }
        // Unclosed, malformed, unknown, repeated, or bodiless configs never dispatch.
        for input in [
            "3`",
            "3`i5",
            "3`i5`",
            "`d5`",
            "3`i`ab",
            "3`d`ab",
            "3`d15`ab",
            "3`i5i1`ab",
            "3`d5i1`ab",
            "3`i5,i1`ab",
            "3`d1,d2`ab",
            "3`d2,`ab",
            "3`,d2`ab",
            "3`d2,,i1`ab",
            "3`x`ab",
            "3`i5`0q",
            "`d5`0q",
        ] {
            assert_eq!(parse(input).unwrap(), ChordCommand::Pending, "{input}");
        }
    }

    #[test]
    fn enter_separates_chords_only_after_a_count_or_config_block() {
        for (input, expected) in [
            (
                r"3`i13`/w]\n/w]",
                run(&["/w]", "/w]"], ChordRepeat::Times(3), &[100, 300]),
            ),
            (
                r"3/w]\n/w]",
                run(&["/w]", "/w]"], ChordRepeat::Times(3), &[]),
            ),
            (r"1ab\ncd", run(&["ab", "cd"], ChordRepeat::Loop, &[])),
            (
                r"`i0`ab\ncd",
                run(&["ab", "cd"], ChordRepeat::Times(1), &[0]),
            ),
            (
                r"`d5`ab\ncd",
                delayed(run(&["ab", "cd"], ChordRepeat::Times(1), &[]), 500),
            ),
            (
                r"2ab\n12cd",
                run(&["ab", "12cd"], ChordRepeat::Times(2), &[]),
            ),
        ] {
            assert_eq!(parse(input).unwrap(), expected, "{input}");
        }
        // Without a count or config block, Enter is just another key in the chord.
        assert_eq!(
            parse(r"/w]\n/w]").unwrap(),
            run(&[r"/w]\n/w]"], ChordRepeat::Times(1), &[])
        );
        assert_eq!(
            parse(r"a\n").unwrap(),
            run(&[r"a\n"], ChordRepeat::Times(1), &[])
        );
        for input in [r"3ab\n", r"3\nab", r"3ab\n\ncd", r"3ab\n0q"] {
            assert_eq!(parse(input).unwrap(), ChordCommand::Pending, "{input}");
        }
    }

    #[test]
    fn quotes_type_their_keys_with_optional_pauses_between_them() {
        for (input, expected) in [
            (
                "'hello'",
                steps(vec![quote("hello", &[])], ChordRepeat::Times(1), &[]),
            ),
            (
                "'hello'`i5`",
                steps(vec![quote("hello", &[500])], ChordRepeat::Times(1), &[]),
            ),
            (
                "'hi'`i10`",
                steps(vec![quote("hi", &[100, 0])], ChordRepeat::Times(1), &[]),
            ),
            (
                "3'hi'",
                steps(vec![quote("hi", &[])], ChordRepeat::Times(3), &[]),
            ),
            // Inside a quote, Enter, backticks, digits and zero are typed keys.
            (
                r"'a\nb`0'",
                steps(vec![quote("a\nb`0", &[])], ChordRepeat::Times(1), &[]),
            ),
            (
                r"2`i9`'hi'`i1`\n/w]\n'yo'",
                steps(
                    vec![quote("hi", &[100]), chord("/w]"), quote("yo", &[])],
                    ChordRepeat::Times(2),
                    &[900],
                ),
            ),
            // A run's delay comes first, however its steps are configured.
            (
                r"`d3`'hi'`i1`",
                delayed(
                    steps(vec![quote("hi", &[100])], ChordRepeat::Times(1), &[]),
                    300,
                ),
            ),
            (
                r"1'a\nb'\nab",
                steps(
                    vec![quote("a\nb", &[]), chord("ab")],
                    ChordRepeat::Loop,
                    &[],
                ),
            ),
            // A quote only starts a step; elsewhere `'` is an ordinary key.
            ("ab'c", run(&["ab'c"], ChordRepeat::Times(1), &[])),
        ] {
            assert_eq!(parse(input).unwrap(), expected, "{input}");
        }
        // Unclosed, empty, delayed, or followed by more than a config block or separator.
        // A step's delay would wait again each loop, so only the run takes `d`.
        for input in [
            "'hello",
            "''",
            "'hi'`i5",
            "'hi'`x`",
            "'hi'`d5`",
            "'hi'`d5,i1`",
            r"2`i1`'hi'`d5`\nab",
            "'hi'x",
            r"'hi'\nab",
            r"3'hi'ab",
            r"3'hi'\n",
        ] {
            assert_eq!(parse(input).unwrap(), ChordCommand::Pending, "{input}");
        }
    }

    #[test]
    fn typed_quotes_take_insert_mode_text_and_an_open_quote_is_insert_mode() {
        // Insert mode keeps the text out of the keys: a typed quote is an adjacent pair.
        for (input, quotes, expected) in [
            (
                "''",
                &["Hello, 'World'!"][..],
                steps(
                    vec![quote("Hello, 'World'!", &[])],
                    ChordRepeat::Times(1),
                    &[],
                ),
            ),
            (
                "3''`i1`",
                &["Hi there"],
                steps(vec![quote("Hi there", &[100])], ChordRepeat::Times(3), &[]),
            ),
            (
                r"2''\n/w]\n'yo'\n''",
                &["A", "B"],
                steps(
                    vec![
                        quote("A", &[]),
                        chord("/w]"),
                        quote("yo", &[]),
                        quote("B", &[]),
                    ],
                    ChordRepeat::Times(2),
                    &[],
                ),
            ),
        ] {
            assert_eq!(parse_typed(input, quotes), expected, "{input}");
        }
        for (input, quotes, index) in [
            ("'", &[][..], 0),
            ("3'", &[""], 0),
            (r"2''\n/w]\n'", &["A", "B"], 1),
            (r"`d5`ab\n'", &[], 0),
        ] {
            assert_eq!(
                parse_typed(input, quotes),
                ChordCommand::Insert { quote: index },
                "{input}"
            );
        }
        // A quote that closed with no text, or a `'` inside a chord, is not insert mode.
        assert_eq!(parse_typed("''", &[""]), ChordCommand::Pending);
        assert_eq!(
            parse_typed("ab'", &[]),
            run(&["ab'"], ChordRepeat::Times(1), &[])
        );
        assert_eq!(
            parse_typed(r"ab\n'", &[]),
            run(&[r"ab\n'"], ChordRepeat::Times(1), &[])
        );
    }
}
