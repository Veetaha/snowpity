#[derive(Debug, thiserror::Error)]
pub enum TemplatePhraseError {
    #[error(
        "Образец фразы должен состоять только из кирилицы и пробелов. \
        Ошибка в символе `{invalid_char}`, фраза: {phrase}"
    )]
    InvalidChar { phrase: String, invalid_char: char },

    #[error(
        "Образец фразы не должен иметь повторяющихся символов. \
        Повторяющийся символ: {repeated_char}, фраза: {phrase}"
    )]
    RepeatedChar { phrase: String, repeated_char: char },

    #[error(
        "Образец фразы не должен превышать {} символов (длина заданной фразы: {}, фраза: {phrase})",
        crate::MAX_TEMPLATE_PHRASE_LEN,
        phrase.len(),
    )]
    TooLong { phrase: String },
}
