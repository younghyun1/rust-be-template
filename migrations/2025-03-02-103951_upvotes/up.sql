CREATE TABLE public.post_upvotes (
 upvote_id uuid DEFAULT uuid_generate_v4() NOT NULL,
 post_id uuid NOT NULL,
 user_id uuid NOT NULL,
 upvoted_at timestamptz DEFAULT now() NOT NULL,
 CONSTRAINT post_upvotes_pkey PRIMARY KEY (upvote_id),
 CONSTRAINT post_upvotes_post_user_unique UNIQUE (post_id, user_id),
 CONSTRAINT fk_post_upvotes_post FOREIGN KEY (post_id) REFERENCES public.posts(post_id) ON DELETE CASCADE,
 CONSTRAINT fk_post_upvotes_user FOREIGN KEY (user_id) REFERENCES public.users(user_id) ON DELETE CASCADE
);

CREATE INDEX idx_post_upvotes_post_id ON public.post_upvotes(post_id);
CREATE INDEX idx_post_upvotes_user_id ON public.post_upvotes(user_id);