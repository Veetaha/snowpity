CREATE TABLE IF NOT EXISTS tg_chat_banned_words(
    tg_chat_id VARCHAR(100) NOT NULL,
    word TEXT NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT (now()::TIMESTAMP),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT (now()::TIMESTAMP),
    created_by VARCHAR(100) NOT NULL,

    CONSTRAINT tg_chat_and_banned_word_composite_pk PRIMARY KEY (tg_chat_id, word),
    CONSTRAINT tg_chats_fk FOREIGN KEY(tg_chat_id) REFERENCES tg_chats(id)
);

-- The expectation is that looking up banned patterns by `tg_chat_id` will dominate
-- the database throughput enough that it's reasonable to make a hash index for it
-- instead of letting it use the default BTree index created for the composite key.
CREATE INDEX tg_chat_banned_words_by_tg_chat_id
    ON tg_chat_banned_words
    USING hash(tg_chat_id);
