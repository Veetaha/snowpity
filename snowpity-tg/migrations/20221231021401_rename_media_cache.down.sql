alter table tg_derpi_media_cache rename constraint tg_derpi_media_cache_pk to tg_media_cache_pk;
alter table tg_derpi_media_cache rename column tg_file_kind to tg_file_type;
alter table tg_derpi_media_cache rename to tg_media_cache;
