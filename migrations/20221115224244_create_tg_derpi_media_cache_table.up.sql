create table if not exists tg_derpi_media_cache(
    -- Derpibooru image ID, although images on derpibooru are not always just
    -- static JPEGs or PNG, the can actually be GIFs or videos, therefore
    -- we call it media instead
    media_id int8 not null,
    tg_file_id varchar(100) not null,

    constraint tg_derpi_media_cache_media_id_pk primary key (media_id)
);
