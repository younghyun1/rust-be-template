-- Drop indexes for the comments table.
DROP INDEX IF EXISTS idx_comments_created_at;

DROP INDEX IF EXISTS idx_comments_updated_at;

DROP INDEX IF EXISTS idx_comments_parent_id;

DROP INDEX IF EXISTS idx_comments_user_id;

DROP INDEX IF EXISTS idx_comments_post_id;

-- Drop the comments table.
DROP TABLE IF EXISTS comments;

-- Drop indexes for the posts table.
DROP INDEX IF EXISTS idx_posts_created_at;

DROP INDEX IF EXISTS idx_posts_updated_at;

DROP INDEX IF EXISTS idx_posts_published_at;

DROP INDEX IF EXISTS idx_posts_user_id;

DROP INDEX IF EXISTS idx_posts_slug;

-- Drop the posts table.
DROP TABLE IF EXISTS posts;
