use std::fmt;

use crate::util::{FromDb, IntoDb};
use crate::{err_val, Result, UserError};
use lazy_regex::regex_is_match;

/// Limit of the text length for the banned word
pub(crate) const MAX_WORD_LENGTH: usize = 60;

#[derive(Debug, Clone)]
pub struct Word(String);

impl Word {
    pub(crate) fn new(word: &str) -> Result<Self> {
        let word = word.trim().to_lowercase();

        if word.len() > MAX_WORD_LENGTH {
            return err_val!(UserError::BannedWordTooLong { word });
        }

        if !regex_is_match!(r"^[\w0-9_\-]+$", &word) {
            return Err(err_val!(UserError::BannedWordMalformed { word }));
        }

        Ok(Self(word))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Word {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl IntoDb<String> for Word {
    fn into_db(self) -> String {
        self.0
    }
}

impl FromDb<String> for Word {
    fn from_db(str: String) -> Word {
        Self::new(&str).unwrap_or_else(|err| {
            panic!("Invalid banned word in database: {str}, {err:#}");
        })
    }
}
