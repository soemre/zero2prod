use rand::{
    distr::{Alphanumeric, SampleString},
    Rng,
};

const ALLOWED_CHARS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ\
abcdefghijklmnopqrstuvwxyz\
0123456789";

const TOKEN_LEN: usize = 25;

#[derive(Debug, Clone)]
pub struct SubscriptionToken(String);

impl SubscriptionToken {
    pub fn parse(s: String) -> Result<Self, String> {
        let is_empty = s.trim().is_empty();
        let is_not_ascii = !s.is_ascii();
        let is_right_sized = s.chars().count() != TOKEN_LEN;
        let contains_forbidden_chars = !s.chars().all(|c| ALLOWED_CHARS.contains(c));

        if is_empty || is_not_ascii || is_right_sized || contains_forbidden_chars {
            Err(format!("{} is not a valid subscriber name.", s))
        } else {
            Ok(Self(s))
        }
    }

    /// Generate a random 25-characters-long case-sensitive subscription token.
    pub fn generate() -> Self {
        Self::generate_with_rng(&mut rand::rng())
    }

    fn generate_with_rng<R: Rng + ?Sized>(rng: &mut R) -> Self {
        let raw = Alphanumeric.sample_string(rng, TOKEN_LEN);
        SubscriptionToken(raw)
    }
}

impl AsRef<str> for SubscriptionToken {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::assert_err;
    use fake::Fake;
    use quickcheck::Arbitrary;
    use quickcheck_macros::quickcheck;
    use rand::{rngs::StdRng, SeedableRng};

    impl Arbitrary for SubscriptionToken {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            let mut rng = StdRng::seed_from_u64(u64::arbitrary(g));
            SubscriptionToken::generate_with_rng(&mut rng)
        }
    }

    #[quickcheck]
    fn generated_tokens_can_be_parsed(token: SubscriptionToken) -> bool {
        SubscriptionToken::parse(token.as_ref().to_owned()).is_ok()
    }

    #[test]
    fn a_token_longer_than_expected_is_rejected() {
        let token =
            Alphanumeric.sample_string(&mut rand::rng(), TOKEN_LEN + (1..10).fake::<usize>());
        assert_err!(SubscriptionToken::parse(token));
    }

    #[test]
    fn a_token_shorter_than_expected_is_rejected() {
        let token =
            Alphanumeric.sample_string(&mut rand::rng(), TOKEN_LEN - (1..25).fake::<usize>());
        assert_err!(SubscriptionToken::parse(token));
    }

    #[test]
    fn whitespace_only_tokens_are_rejected() {
        let token = " ".repeat(TOKEN_LEN);
        assert_err!(SubscriptionToken::parse(token));
    }

    #[test]
    fn empty_string_is_rejected() {
        let token = "".to_string();
        assert_err!(SubscriptionToken::parse(token));
    }

    #[quickcheck]
    fn only_the_tokens_containing_non_allowed_chars_are_rejected(s: String) -> bool {
        if s.chars().count() == TOKEN_LEN && s.chars().all(|c| ALLOWED_CHARS.contains(c)) {
            return SubscriptionToken::parse(s).is_ok();
        }
        SubscriptionToken::parse(s).is_err()
    }
}
