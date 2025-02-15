use std::convert::AsRef;
use unicode_segmentation::UnicodeSegmentation;

const FORBIDDEN_CHARS: [char; 9] = ['/', '(', ')', '"', '<', '>', '\\', '{', '}'];

pub struct NewSubscriber {
    pub email: String,
    pub name: SubscriberName,
}

#[derive(Debug)]
pub struct SubscriberName(String);

impl SubscriberName {
    pub fn inner(self) -> String {
        self.0
    }

    pub fn parse(s: &str) -> Result<SubscriberName, String> {
        let is_empty = s.trim().is_empty();

        let is_too_long = s.graphemes(true).count() > 256;

        let contains_forbidden_chars = s.chars().any(|c| FORBIDDEN_CHARS.contains(&c));

        if is_empty || is_too_long || contains_forbidden_chars {
            Err(format!("{} is not a valid subscriber name.", s))
        } else {
            Ok(Self(s.to_string()))
        }
    }
}

impl AsRef<str> for SubscriberName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_err, assert_ok};

    #[test]
    fn a_256_grapheme_long_name_is_valid() {
        let name = "ё".repeat(256);
        assert_ok!(SubscriberName::parse(&name));
    }

    #[test]
    fn a_name_longer_than_256_graphemes_is_rejected() {
        let name = "ё".repeat(257);
        assert_err!(SubscriberName::parse(&name));
    }

    #[test]
    fn whitespace_only_names_are_rejected() {
        let name = " ";
        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn empty_string_is_rejected() {
        let name = "";
        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn names_containing_an_invalid_character_are_rejected() {
        for name in FORBIDDEN_CHARS {
            let name = name.to_string();
            assert_err!(SubscriberName::parse(&name));
        }
    }

    #[test]
    fn a_valid_name_is_parsed_successfully() {
        let name = "Ursula Le Guin";
        assert_ok!(SubscriberName::parse(name));
    }
}
