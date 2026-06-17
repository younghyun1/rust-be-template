------------------------------------------------------------
-- Photograph social: view counts, votes, comments, comment votes.
-- Mirrors the blog social schema with table-prefixed column names.
------------------------------------------------------------

-- 1. Denormalized counters on photographs (naive +1 view, vote totals).
ALTER TABLE photographs
    ADD COLUMN photograph_view_count bigint NOT NULL DEFAULT 0,
    ADD COLUMN photograph_total_upvotes bigint NOT NULL DEFAULT 0,
    ADD COLUMN photograph_total_downvotes bigint NOT NULL DEFAULT 0;

CREATE INDEX idx_photographs_view_count ON photographs USING btree (photograph_view_count DESC);
CREATE INDEX idx_photographs_total_upvotes ON photographs USING btree (photograph_total_upvotes DESC);
CREATE INDEX idx_photographs_total_downvotes ON photographs USING btree (photograph_total_downvotes DESC);

-- 2. Photograph votes (one row per user per photograph).
CREATE TABLE photograph_votes (
    photograph_vote_id uuid PRIMARY KEY DEFAULT uuidv7(),
    photograph_id uuid NOT NULL,
    user_id uuid NOT NULL,
    photograph_vote_created_at timestamptz NOT NULL DEFAULT now(),
    is_upvote boolean NOT NULL,
    CONSTRAINT fk_photograph_votes_photograph FOREIGN KEY (photograph_id) REFERENCES photographs (photograph_id) ON DELETE CASCADE,
    CONSTRAINT fk_photograph_votes_user FOREIGN KEY (user_id) REFERENCES users (user_id) ON DELETE CASCADE,
    CONSTRAINT photograph_votes_photograph_user_unique UNIQUE (photograph_id, user_id)
);

CREATE INDEX idx_photograph_votes_photograph_id ON photograph_votes USING btree (photograph_id);
CREATE INDEX idx_photograph_votes_user_id ON photograph_votes USING btree (user_id);
CREATE INDEX idx_photograph_votes_is_upvote ON photograph_votes USING btree (is_upvote);

-- 3. Photograph comments (threaded via self-referential parent FK).
CREATE TABLE photograph_comments (
    photograph_comment_id uuid PRIMARY KEY DEFAULT uuidv7(),
    photograph_id uuid NOT NULL,
    user_id uuid NOT NULL,
    photograph_comment_content text NOT NULL,
    photograph_comment_created_at timestamptz NOT NULL DEFAULT now(),
    photograph_comment_updated_at timestamptz,
    parent_photograph_comment_id uuid,
    photograph_comment_total_upvotes bigint NOT NULL DEFAULT 0,
    photograph_comment_total_downvotes bigint NOT NULL DEFAULT 0,
    CONSTRAINT fk_photograph_comments_photograph FOREIGN KEY (photograph_id) REFERENCES photographs (photograph_id) ON DELETE CASCADE,
    CONSTRAINT fk_photograph_comments_user FOREIGN KEY (user_id) REFERENCES users (user_id) ON DELETE CASCADE,
    CONSTRAINT fk_photograph_comments_parent FOREIGN KEY (parent_photograph_comment_id) REFERENCES photograph_comments (photograph_comment_id) ON DELETE CASCADE
);

CREATE INDEX idx_photograph_comments_photograph_id ON photograph_comments USING btree (photograph_id);
CREATE INDEX idx_photograph_comments_user_id ON photograph_comments USING btree (user_id);
CREATE INDEX idx_photograph_comments_parent_id ON photograph_comments USING btree (parent_photograph_comment_id);
CREATE INDEX idx_photograph_comments_created_at ON photograph_comments USING btree (photograph_comment_created_at);

-- 4. Photograph comment votes (one row per user per comment).
CREATE TABLE photograph_comment_votes (
    photograph_comment_vote_id uuid PRIMARY KEY DEFAULT uuidv7(),
    photograph_comment_id uuid NOT NULL,
    user_id uuid NOT NULL,
    photograph_comment_vote_created_at timestamptz NOT NULL DEFAULT now(),
    is_upvote boolean NOT NULL,
    CONSTRAINT fk_photograph_comment_votes_comment FOREIGN KEY (photograph_comment_id) REFERENCES photograph_comments (photograph_comment_id) ON DELETE CASCADE,
    CONSTRAINT fk_photograph_comment_votes_user FOREIGN KEY (user_id) REFERENCES users (user_id) ON DELETE CASCADE,
    CONSTRAINT photograph_comment_votes_comment_user_unique UNIQUE (photograph_comment_id, user_id)
);

CREATE INDEX idx_photograph_comment_votes_comment_id ON photograph_comment_votes USING btree (photograph_comment_id);
CREATE INDEX idx_photograph_comment_votes_user_id ON photograph_comment_votes USING btree (user_id);
CREATE INDEX idx_photograph_comment_votes_is_upvote ON photograph_comment_votes USING btree (is_upvote);
