use crate::{Keys, PreparedFmt, Segment, Segments};
use std::ops::Range;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParsingError {
    #[error("Bad key character '{}' at position {}", .1, .0)]
    BadKeyChar(usize, char),
    #[error("Empty key at position .0")]
    EmptyKey(usize),
    #[error("Unmatched closing brace '}}' at position {}. Use '}}}}' if you want to output '}}' character", .0
    )]
    UnmatchedClosingBrace(usize),
    #[error("Unmatched opening brace '{{' at position {}. Use '{{{{' if you want to output '{{'", .0)]
    UnmatchedOpeningBrace(usize),
}

impl PreparedFmt {
    pub fn parse(fmt: &str) -> Result<Self, ParsingError> {
        let mut segments = Segments::new();
        let mut keys = Keys::new();
        let mut cur_range_start = 0usize;
        let mut in_key = false;

        let mut bad_key_char = None::<(usize, char)>;

        #[inline(always)]
        fn push_literal(
            segments: &mut Segments,
            fmt: &str,
            range: Range<usize>,
        ) -> Result<(), ParsingError> {
            if range.is_empty() {
                return Ok(());
            }

            debug_assert!(range.end <= fmt.len());
            if let Some(Segment::Literal(str)) = segments.last_mut() {
                *str += &fmt[range];
            } else {
                segments.push(Segment::Literal(fmt[range].to_string()));
            }

            Ok(())
        }

        #[inline(always)]
        fn push_key(
            segments: &mut Segments,
            keys: &mut Keys,
            fmt: &str,
            range: Range<usize>,
        ) -> Result<(), ParsingError> {
            if range.is_empty() {
                return Err(ParsingError::EmptyKey(range.start));
            }

            debug_assert!(range.end <= fmt.len());
            let key = fmt[range].to_string();

            if !keys.contains(&key) {
                keys.push(key.clone());
            }

            segments.push(Segment::Key(key));

            Ok(())
        }

        let mut chars = fmt.char_indices().peekable();
        while let Some((idx, char)) = chars.next() {
            match (in_key, char) {
                // Opening or escaped {
                (false, '{') => {
                    // handle {{ to be parsed as two string segments to produce single { in output
                    // without allocations
                    if chars.peek().is_some_and(|c| c.1 == '{') {
                        push_literal(&mut segments, fmt, cur_range_start..idx)?;
                        cur_range_start = idx + 1;
                        // skip the second {
                        chars.next();
                        continue;
                    }

                    push_literal(&mut segments, fmt, cur_range_start..idx)?;
                    cur_range_start = idx + 1;

                    in_key = true;
                }
                // Closing } or key chars
                (true, char) => {
                    if char == '}' {
                        push_key(&mut segments, &mut keys, fmt, cur_range_start..idx)?;
                        cur_range_start = idx + 1;
                        in_key = false;
                    } else if !char.is_ascii_alphanumeric() && char != '_' {
                        if char == '{' {
                            return Err(ParsingError::UnmatchedOpeningBrace(cur_range_start - 1));
                        }
                        // insert the char if it's the first occurrence
                        bad_key_char.get_or_insert((idx, char));

                        // do not error immediately. Error might have been caused by the unmatched
                        // opening brace, which has higher priority but cannot be caught without
                        // forward-parsing
                    }
                }
                // Escaped }
                (false, '}') => {
                    if chars.peek().is_none_or(|c| c.1 != '}') {
                        return Err(ParsingError::UnmatchedClosingBrace(idx));
                    } else {
                        push_literal(&mut segments, fmt, cur_range_start..idx)?;
                        cur_range_start = idx + 1;
                        chars.next();
                    }
                }
                // Any non-special char outside the key
                (false, _) => {}
            }
        }

        if in_key {
            return Err(ParsingError::UnmatchedOpeningBrace(cur_range_start - 1));
        } else if let Some((idx, char)) = bad_key_char {
            return Err(ParsingError::BadKeyChar(idx, char));
        } else {
            push_literal(&mut segments, fmt, cur_range_start..fmt.len())?;
        }

        Ok(Self { segments, keys })
    }
}
