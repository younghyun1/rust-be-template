ALTER TABLE public.posts
    ADD COLUMN post_view_count int8 DEFAULT 0 NOT NULL,
    ADD COLUMN post_upvote_count int8 DEFAULT 0 NOT NULL,
    ADD COLUMN post_share_count int8 DEFAULT 0 NOT NULL,
    ADD COLUMN post_metadata jsonb DEFAULT '{}'::jsonb NOT NULL;

-- Add indexes for efficient querying of popular posts
CREATE INDEX idx_posts_view_count ON public.posts USING btree (post_view_count DESC);
CREATE INDEX idx_posts_upvote_count ON public.posts USING btree (post_upvote_count DESC);
