DROP INDEX IF EXISTS idx_posts_upvote_count;

ALTER TABLE public.posts
DROP COLUMN post_upvote_count;
