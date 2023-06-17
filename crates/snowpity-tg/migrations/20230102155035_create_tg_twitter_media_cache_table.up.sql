create table if not exists tg_twitter_media_cache(
    tweet_id bigint not null,
    media_key varchar(100) not null,
    tg_file_id varchar(100) not null,
    tg_file_kind smallint not null,

    constraint tg_twitter_media_cache_pk primary key (tweet_id, media_key)

    -- Technically, tg_file_id must be unique, but it's not enforced
    -- just to save memory. We don't want to have an index to validate an
    -- invariant, that we control and we make sure to never violate it.
    -- constraint tg_file_id_unique unique (tg_file_id)
);
