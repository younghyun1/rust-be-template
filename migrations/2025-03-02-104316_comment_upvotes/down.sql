DROP INDEX IF EXISTS idx_comment_upvotes_comment_id;

DROP INDEX IF EXISTS idx_comment_upvotes_user_id;

DROP INDEX IF EXISTS idx_comment_upvotes_created_at;

DROP TABLE IF EXISTS public."comment_upvotes" CASCADE;
