-- Drop table for post-to-tag relations first (due to foreign key constraints)
DROP TABLE IF EXISTS public.post_tags;

-- Drop table for tags
DROP TABLE IF EXISTS public.tags;
