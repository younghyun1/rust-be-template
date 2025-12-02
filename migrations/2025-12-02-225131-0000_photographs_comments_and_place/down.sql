DROP INDEX IF EXISTS photographs_lat_idx;
DROP INDEX IF EXISTS photographs_lon_idx;

ALTER TABLE public.photographs
 DROP COLUMN IF EXISTS photograph_comments,
 DROP COLUMN IF EXISTS photograph_lat,
 DROP COLUMN IF EXISTS photograph_lon;
