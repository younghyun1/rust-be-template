ALTER TABLE public.posts ADD COLUMN post_upvote_count int8 DEFAULT 0 NOT NULL;
CREATE INDEX idx_posts_upvote_count ON public.posts USING btree (post_upvote_count DESC);
