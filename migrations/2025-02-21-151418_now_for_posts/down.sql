-- This file should undo anything in `up.sql`
ALTER TABLE public.posts
ALTER COLUMN post_created_at
DROP DEFAULT,
ALTER COLUMN post_updated_at
DROP DEFAULT;
