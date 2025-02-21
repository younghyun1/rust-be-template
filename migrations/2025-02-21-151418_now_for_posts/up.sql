-- Your SQL goes here
ALTER TABLE public.posts 
    ALTER COLUMN post_created_at SET DEFAULT now(),
    ALTER COLUMN post_updated_at SET DEFAULT now();