-- Comment upvotes table definition
CREATE TABLE public."comment_upvotes" (
 upvote_id uuid DEFAULT uuid_generate_v4() NOT NULL,
 comment_id uuid NOT NULL,
 user_id uuid NOT NULL,
 created_at timestamptz DEFAULT now() NOT NULL,
 CONSTRAINT comment_upvotes_pkey PRIMARY KEY (upvote_id),
 CONSTRAINT fk_comment_upvotes_comment FOREIGN KEY (comment_id) REFERENCES public."comments"(comment_id) ON DELETE CASCADE,
 CONSTRAINT fk_comment_upvotes_user FOREIGN KEY (user_id) REFERENCES public.users(user_id) ON DELETE CASCADE,
 CONSTRAINT unique_user_comment_upvote UNIQUE (user_id, comment_id)
);
CREATE INDEX idx_comment_upvotes_comment_id ON public.comment_upvotes USING btree (comment_id);
CREATE INDEX idx_comment_upvotes_user_id ON public.comment_upvotes USING btree (user_id);
CREATE INDEX idx_comment_upvotes_created_at ON public.comment_upvotes USING btree (created_at);