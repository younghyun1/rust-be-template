-- Create table for tags
CREATE TABLE public.tags (
    tag_id smallint GENERATED ALWAYS AS IDENTITY,
    tag_name varchar NOT NULL UNIQUE,
    CONSTRAINT tags_pkey PRIMARY KEY (tag_id)
);

-- Create table for post-to-tag relations (many-to-many)
CREATE TABLE public.post_tags (
    post_id uuid NOT NULL,
    tag_id smallint NOT NULL,
    CONSTRAINT post_tags_pkey PRIMARY KEY (post_id, tag_id),
    CONSTRAINT fk_post_tags_post FOREIGN KEY (post_id) REFERENCES public.posts(post_id) ON DELETE CASCADE,
    CONSTRAINT fk_post_tags_tag FOREIGN KEY (tag_id) REFERENCES public.tags(tag_id) ON DELETE CASCADE
);
