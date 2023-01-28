alter table tg_derpi_media_cache rename to tg_derpibooru_blob_cache;

alter table tg_derpibooru_blob_cache
rename constraint tg_derpi_media_cache_pk to tg_derpibooru_blob_cache_pk;

alter table tg_derpibooru_blob_cache
rename column derpi_id to derpibooru_id;

alter table tg_twitter_media_cache rename to tg_twitter_blob_cache;

alter table tg_twitter_blob_cache
rename constraint tg_twitter_media_cache_pk to tg_twitter_blob_cache_pk;
