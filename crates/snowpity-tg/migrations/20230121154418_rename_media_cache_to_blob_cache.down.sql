alter table tg_twitter_blob_cache
rename constraint tg_twitter_blob_cache_pk to tg_twitter_media_cache_pk;

alter table tg_twitter_blob_cache rename to tg_twitter_media_cache;

alter table tg_derpibooru_blob_cache
rename column derpibooru_id to derpi_id;

alter table tg_derpibooru_blob_cache
rename constraint tg_derpibooru_blob_cache_pk to tg_derpi_media_cache_pk;

alter table tg_derpibooru_blob_cache rename to tg_derpi_media_cache;
