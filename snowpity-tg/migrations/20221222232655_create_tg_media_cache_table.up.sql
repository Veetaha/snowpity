create table if not exists tg_media_cache(
    derpi_id bigint not null,
    tg_file_id varchar(100) not null,
    tg_file_type smallint not null,

    constraint tg_media_cache_pk primary key (derpi_id),
    constraint tg_file_id_unique unique (tg_file_id)
);
