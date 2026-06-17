-- Reverse the photograph social schema.
DROP TABLE IF EXISTS photograph_comment_votes;
DROP TABLE IF EXISTS photograph_comments;
DROP TABLE IF EXISTS photograph_votes;

DROP INDEX IF EXISTS idx_photographs_total_downvotes;
DROP INDEX IF EXISTS idx_photographs_total_upvotes;
DROP INDEX IF EXISTS idx_photographs_view_count;

ALTER TABLE photographs
    DROP COLUMN IF EXISTS photograph_total_downvotes,
    DROP COLUMN IF EXISTS photograph_total_upvotes,
    DROP COLUMN IF EXISTS photograph_view_count;
