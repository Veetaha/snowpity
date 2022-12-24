create table if not exists tg_media_cache(
    derpi_id bigint not null,
    tg_file_id varchar(100) not null,
    tg_file_type smallint not null,

    constraint tg_media_cache_pk primary key (derpi_id)

    -- Technically, tg_file_id must be unique, but it's not enforced
    -- just to save memory. We don't want to have an index to validate an
    -- invariant, that we control and we make sure to never violate it.
    -- constraint tg_file_id_unique unique (tg_file_id)
);
