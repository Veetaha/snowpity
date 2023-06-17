create table if not exists tg_deviant_art_blob_cache(
    deviation_numeric_id bigint not null,
    tg_file_id varchar(100) not null,
    tg_file_kind smallint not null,

    constraint tg_deviant_art_blob_cache_pk primary key (deviation_numeric_id)
);
