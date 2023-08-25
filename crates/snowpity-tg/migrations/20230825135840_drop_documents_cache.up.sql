-- Delete all documents from the cache, because now we resize images to fit to the
-- Telegram `width * height` limt, so we want to regenerate all images that were
-- potentially uploaded as documents into the cache
delete from tg_derpibooru_blob_cache
where tg_file_kind = 1;

delete from tg_deviant_art_blob_cache
where tg_file_kind = 1;

delete from tg_twitter_blob_cache
where tg_file_kind = 1;
