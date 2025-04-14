-- 1. Rename tables
ALTER TABLE public.comment_upvotes
RENAME TO comment_votes;

ALTER TABLE public.post_upvotes
RENAME TO post_votes;

-- 2. Add is_upvote field (default true since these were originally upvotes)
ALTER TABLE public.comment_votes
ADD COLUMN is_upvote boolean NOT NULL DEFAULT true;

ALTER TABLE public.post_votes
ADD COLUMN is_upvote boolean NOT NULL DEFAULT true;

-- 3. Add indexes for is_upvote
CREATE INDEX idx_comment_votes_is_upvote ON public.comment_votes USING btree (is_upvote);

CREATE INDEX idx_post_votes_is_upvote ON public.post_votes USING btree (is_upvote);

-- 4. Add denormalized counters to posts and comments
ALTER TABLE public.posts
ADD COLUMN total_upvotes bigint NOT NULL DEFAULT 0;

ALTER TABLE public.posts
ADD COLUMN total_downvotes bigint NOT NULL DEFAULT 0;

ALTER TABLE public.comments
ADD COLUMN total_upvotes bigint NOT NULL DEFAULT 0;

ALTER TABLE public.comments
ADD COLUMN total_downvotes bigint NOT NULL DEFAULT 0;

-- 5. Add indexes for the counters
CREATE INDEX idx_posts_total_upvotes ON public.posts USING btree (total_upvotes DESC);

CREATE INDEX idx_posts_total_downvotes ON public.posts USING btree (total_downvotes DESC);

CREATE INDEX idx_comments_total_upvotes ON public.comments USING btree (total_upvotes DESC);

CREATE INDEX idx_comments_total_downvotes ON public.comments USING btree (total_downvotes DESC);

-- 6. Rename primary key columns
ALTER TABLE public.comment_votes
RENAME COLUMN upvote_id TO vote_id;

ALTER TABLE public.post_votes
RENAME COLUMN upvote_id TO vote_id;

ALTER TABLE public.post_votes
RENAME COLUMN upvoted_at TO created_at;

-- 7. Rename constraints
ALTER TABLE public.comment_votes RENAME CONSTRAINT comment_upvotes_pkey TO comment_votes_pkey;

ALTER TABLE public.comment_votes RENAME CONSTRAINT unique_user_comment_upvote TO unique_user_comment_vote;

ALTER TABLE public.comment_votes RENAME CONSTRAINT fk_comment_upvotes_comment TO fk_comment_votes_comment;

ALTER TABLE public.comment_votes RENAME CONSTRAINT fk_comment_upvotes_user TO fk_comment_votes_user;

ALTER TABLE public.post_votes RENAME CONSTRAINT post_upvotes_pkey TO post_votes_pkey;

ALTER TABLE public.post_votes RENAME CONSTRAINT post_upvotes_post_user_unique TO post_votes_post_user_unique;

ALTER TABLE public.post_votes RENAME CONSTRAINT fk_post_upvotes_post TO fk_post_votes_post;

ALTER TABLE public.post_votes RENAME CONSTRAINT fk_post_upvotes_user TO fk_post_votes_user;

-- 8. Rename indexes
ALTER INDEX idx_comment_upvotes_comment_id
RENAME TO idx_comment_votes_comment_id;

ALTER INDEX idx_comment_upvotes_created_at
RENAME TO idx_comment_votes_created_at;

ALTER INDEX idx_comment_upvotes_user_id
RENAME TO idx_comment_votes_user_id;

ALTER INDEX idx_post_upvotes_post_id
RENAME TO idx_post_votes_post_id;

ALTER INDEX idx_post_upvotes_user_id
RENAME TO idx_post_votes_user_id;

-- 9. Initialize denormalized counters based on current votes
UPDATE public.posts p
SET
    total_upvotes = (
        SELECT
            COUNT(*)
        FROM
            public.post_votes pv
        WHERE
            pv.post_id = p.post_id
            AND pv.is_upvote = true
    );

UPDATE public.comments c
SET
    total_upvotes = (
        SELECT
            COUNT(*)
        FROM
            public.comment_votes cv
        WHERE
            cv.comment_id = c.comment_id
            AND cv.is_upvote = true
    );
