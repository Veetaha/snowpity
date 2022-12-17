use crate::TemplatePhraseError;
use itertools::Itertools;
use std::fmt;

pub(crate) const MAX_TEMPLATE_PHRASE_LEN: usize = 60;

#[derive(Debug, Clone)]
pub struct TemplatePhrase(String);

impl TemplatePhrase {
    pub fn new(phrase: &str) -> Result<Self, TemplatePhraseError> {
        // Normalize whitespace and casing in the phrase
        let phrase = phrase.to_lowercase().split_whitespace().join(" ");

        if phrase.len() > MAX_TEMPLATE_PHRASE_LEN {
            return Err(TemplatePhraseError::TooLong { phrase });
        }

        let repeated = phrase
            .chars()
            .tuple_windows()
            .find(|(cur, next)| cur == next);

        if let Some((repeated_char, _)) = repeated {
            return Err(TemplatePhraseError::RepeatedChar {
                phrase,
                repeated_char,
            });
        }

        Ok(Self(phrase))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for TemplatePhrase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}
