alter table tg_media_cache rename to tg_derpi_media_cache;
alter table tg_derpi_media_cache rename column tg_file_type to tg_file_kind;
alter table tg_derpi_media_cache rename constraint tg_media_cache_pk to tg_derpi_media_cache_pk;
