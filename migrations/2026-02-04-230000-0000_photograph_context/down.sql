DROP INDEX IF EXISTS photographs_context_idx;
ALTER TABLE public.photographs
    DROP CONSTRAINT IF EXISTS photographs_context_check;
ALTER TABLE public.photographs
    DROP COLUMN IF EXISTS photograph_context;
