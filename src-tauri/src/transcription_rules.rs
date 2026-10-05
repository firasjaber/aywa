use regex::{Regex, RegexBuilder};
use std::sync::LazyLock;

use crate::settings::TranscriptionRule;

pub fn apply_rules(text: &str, rules: &[TranscriptionRule]) -> String {
    if rules.is_empty() {
        return text.to_string();
    }

    let matchers = rules
        .iter()
        .enumerate()
        .filter_map(|(index, rule)| {
            RegexBuilder::new(&regex::escape(&rule.heard_as))
                .case_insensitive(true)
                .build()
                .ok()
                .map(|matcher| (index, matcher))
        })
        .collect::<Vec<_>>();
    let mut output = String::with_capacity(text.len());
    let mut cursor = 0;
    let mut deleted = false;

    while cursor < text.len() {
        let next = matchers
            .iter()
            .filter_map(|(index, matcher)| {
                find_whole_match(matcher, text, cursor).map(|span| (*index, span))
            })
            .min_by(
                |(_, (left_start, left_end)), (_, (right_start, right_end))| {
                    left_start
                        .cmp(right_start)
                        .then_with(|| right_end.cmp(left_end))
                },
            );
        let Some((index, (start, end))) = next else {
            break;
        };

        output.push_str(&text[cursor..start]);
        let rule = &rules[index];
        if rule.use_instead.is_empty() {
            deleted = true;
            let trimmed = output.trim_end_matches([' ', '\t']);
            let had_space = trimmed.len() != output.len();
            output.truncate(trimmed.len());
            let mut removed_separator = false;
            if output.ends_with(&[',', ';', ':'][..]) {
                output.pop();
                removed_separator = true;
            }
            cursor = end;
            if text[cursor..].starts_with(&[',', ';', ':'][..]) {
                cursor += 1;
                removed_separator = true;
            }
            if output.trim().is_empty() {
                while let Some(character) = text[cursor..].chars().next() {
                    if !is_punctuation(character) {
                        break;
                    }
                    cursor += character.len_utf8();
                }
            }
            let space_start = cursor;
            while text[cursor..].starts_with([' ', '\t']) {
                cursor += 1;
            }
            if let (Some(before), Some(after)) =
                (output.chars().last(), text[cursor..].chars().next())
            {
                let separated = had_space || cursor != space_start || removed_separator;
                let closes_text =
                    matches!(after, '.' | ',' | ';' | ':' | '!' | '?' | ')' | ']' | '}');
                let opens_text = matches!(before, '(' | '[' | '{');
                if !before.is_whitespace()
                    && !after.is_whitespace()
                    && !closes_text
                    && !opens_text
                    && (separated || (is_word_char(before) && is_word_char(after)))
                {
                    output.push(' ');
                }
            }
        } else {
            output.push_str(&rule.use_instead);
            cursor = end;
        }
    }
    output.push_str(&text[cursor..]);

    if deleted
        && output
            .chars()
            .all(|character| character.is_whitespace() || is_punctuation(character))
    {
        String::new()
    } else {
        output
    }
}

fn find_whole_match(matcher: &Regex, text: &str, mut offset: usize) -> Option<(usize, usize)> {
    while let Some(found) = matcher.find_at(text, offset) {
        if found.is_empty() {
            return None;
        }
        let before = text[..found.start()].chars().next_back();
        let after = text[found.end()..].chars().next();
        if !before.is_some_and(is_word_char) && !after.is_some_and(is_word_char) {
            return Some((found.start(), found.end()));
        }
        offset = found.start() + text[found.start()..].chars().next()?.len_utf8();
    }
    None
}

fn is_word_char(character: char) -> bool {
    static WORD_CHAR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\w$").unwrap());
    WORD_CHAR.is_match(character.encode_utf8(&mut [0; 4]))
}

fn is_punctuation(character: char) -> bool {
    static PUNCTUATION: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\p{P}$").unwrap());
    PUNCTUATION.is_match(character.encode_utf8(&mut [0; 4]))
}

#[cfg(test)]
mod tests {
    use super::apply_rules;
    use crate::settings::TranscriptionRule;

    fn rule(heard_as: &str, use_instead: &str) -> TranscriptionRule {
        TranscriptionRule {
            heard_as: heard_as.to_string(),
            use_instead: use_instead.to_string(),
        }
    }

    #[test]
    fn corrects_complete_words_with_literal_case_and_unicode_matching() {
        let rules = [rule("hawfa", "Haufe"), rule("МОСКВА", "Moscow")];
        assert_eq!(
            apply_rules("Hawfa met МОСКВА, but preHawfa did not.", &rules),
            "Haufe met Moscow, but preHawfa did not."
        );
    }

    #[test]
    fn combining_marks_keep_a_partial_match_inside_the_word() {
        assert_eq!(apply_rules("مرحَبا", &[rule("مرح", "X")]), "مرحَبا");
    }

    #[test]
    fn deletes_words_and_takes_attached_commas() {
        let rules = [rule("uhm", "")];
        assert_eq!(apply_rules("Uhm, I agree", &rules), "I agree");
        assert_eq!(apply_rules("I, uhm, agree", &rules), "I agree");
        assert_eq!(apply_rules("I said uhm.", &rules), "I said.");
        assert_eq!(apply_rules("uhm.", &rules), "");
        assert_eq!(apply_rules("I,uhm,agree", &rules), "I agree");
        assert_eq!(apply_rules("uhm。", &rules), "");
        assert_eq!(apply_rules("I uhm \"agree\"", &rules), "I \"agree\"");
    }

    #[test]
    fn deletion_keeps_unrelated_paragraph_breaks() {
        let text = "First paragraph.\n\nSecond paragraph says uhm hello.";
        assert_eq!(
            apply_rules(text, &[rule("uhm", "")]),
            "First paragraph.\n\nSecond paragraph says hello."
        );
    }

    #[test]
    fn longest_match_wins_without_cascading_replacements() {
        let rules = [
            rule("new", "old"),
            rule("new york", "New York City"),
            rule("New York City", "NYC"),
        ];
        assert_eq!(apply_rules("new york", &rules), "New York City");
    }
}
