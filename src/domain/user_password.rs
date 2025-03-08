use secrecy::{ExposeSecret, SecretString};
use std::fmt::Debug;

const MIN_PASSWORD_LENGTH: usize = 12;
const MAX_PASSWORD_LENGTH: usize = 129;

pub struct ValidPassword(SecretString);

impl ValidPassword {
    pub fn parse(s: SecretString) -> Result<Self, ValidPasswordError> {
        if !((MIN_PASSWORD_LENGTH + 1)..MAX_PASSWORD_LENGTH).contains(&s.expose_secret().len()) {
            return Err(ValidPasswordError::InvalidLength);
        }

        Ok(Self(s))
    }

    pub fn inner(self) -> SecretString {
        self.0
    }
}

impl AsRef<SecretString> for ValidPassword {
    fn as_ref(&self) -> &SecretString {
        &self.0
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ValidPasswordError {
    #[error(
        "Passwords must be longer than {} characters but shorter than {} characters.",
        MIN_PASSWORD_LENGTH,
        MAX_PASSWORD_LENGTH
    )]
    InvalidLength,
}
