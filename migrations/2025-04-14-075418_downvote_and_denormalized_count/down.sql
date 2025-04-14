-- 1. Remove denormalized counters
ALTER TABLE public.posts
DROP COLUMN total_upvotes;

ALTER TABLE public.posts
DROP COLUMN total_downvotes;

ALTER TABLE public.comments
DROP COLUMN total_upvotes;

ALTER TABLE public.comments
DROP COLUMN total_downvotes;

-- 2. Rename indexes back
ALTER INDEX idx_comment_votes_comment_id
RENAME TO idx_comment_upvotes_comment_id;

ALTER INDEX idx_comment_votes_created_at
RENAME TO idx_comment_upvotes_created_at;

ALTER INDEX idx_comment_votes_user_id
RENAME TO idx_comment_upvotes_user_id;

ALTER INDEX idx_post_votes_post_id
RENAME TO idx_post_upvotes_post_id;

ALTER INDEX idx_post_votes_user_id
RENAME TO idx_post_upvotes_user_id;

-- 3. Rename constraints back
ALTER TABLE public.comment_votes RENAME CONSTRAINT comment_votes_pkey TO comment_upvotes_pkey;

ALTER TABLE public.comment_votes RENAME CONSTRAINT unique_user_comment_vote TO unique_user_comment_upvote;

ALTER TABLE public.comment_votes RENAME CONSTRAINT fk_comment_votes_comment TO fk_comment_upvotes_comment;

ALTER TABLE public.comment_votes RENAME CONSTRAINT fk_comment_votes_user TO fk_comment_upvotes_user;

ALTER TABLE public.post_votes RENAME CONSTRAINT post_votes_pkey TO post_upvotes_pkey;

ALTER TABLE public.post_votes RENAME CONSTRAINT post_votes_post_user_unique TO post_upvotes_post_user_unique;

ALTER TABLE public.post_votes RENAME CONSTRAINT fk_post_votes_post TO fk_post_upvotes_post;

ALTER TABLE public.post_votes RENAME CONSTRAINT fk_post_votes_user TO fk_post_upvotes_user;

-- 4. Rename primary key columns back
ALTER TABLE public.comment_votes
RENAME COLUMN vote_id TO upvote_id;

ALTER TABLE public.post_votes
RENAME COLUMN vote_id TO upvote_id;

ALTER TABLE public.post_votes
RENAME COLUMN created_at TO upvoted_at;

-- 5. Remove indexes for the new is_upvote field
DROP INDEX idx_comment_votes_is_upvote;

DROP INDEX idx_post_votes_is_upvote;

DROP INDEX idx_posts_total_upvotes;

DROP INDEX idx_posts_total_downvotes;

DROP INDEX idx_comments_total_upvotes;

DROP INDEX idx_comments_total_downvotes;

-- 6. Remove is_upvote field and keep only upvotes
DELETE FROM public.comment_votes
WHERE
    is_upvote = false;

DELETE FROM public.post_votes
WHERE
    is_upvote = false;

ALTER TABLE public.comment_votes
DROP COLUMN is_upvote;

ALTER TABLE public.post_votes
DROP COLUMN is_upvote;

-- 7. Rename tables back
ALTER TABLE public.comment_votes
RENAME TO comment_upvotes;

ALTER TABLE public.post_votes
RENAME TO post_upvotes;
