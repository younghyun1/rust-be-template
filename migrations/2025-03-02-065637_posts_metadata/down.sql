-- This file should undo anything in `up.sql`
DROP INDEX IF EXISTS idx_posts_upvote_count;
DROP INDEX IF EXISTS idx_posts_view_count;

ALTER TABLE public.posts
    DROP COLUMN IF EXISTS post_metadata,
    DROP COLUMN IF EXISTS post_share_count,
    DROP COLUMN IF EXISTS post_upvote_count,
    DROP COLUMN IF EXISTS post_view_count;
