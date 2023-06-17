-- Delete all gifs from the cache, because now we upload them as MP4s
delete from tg_media_cache
where tg_file_type = 3
