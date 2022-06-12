CREATE TABLE IF NOT EXISTS tg_chats(
    id VARCHAR(100) NOT NULL,

    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT (now()::TIMESTAMP),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT (now()::TIMESTAMP),
    created_by VARCHAR(100) NOT NULL,

    banned_pattern_mute_duration INTERVAL,

    CONSTRAINT tg_chats_pk PRIMARY KEY (id)
);
