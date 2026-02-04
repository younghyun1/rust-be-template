DO $$
BEGIN
    CREATE TYPE public.photograph_context AS ENUM ('photography', 'post');
EXCEPTION
    WHEN duplicate_object THEN NULL;
END $$;

DROP INDEX IF EXISTS photographs_context_idx;
ALTER TABLE public.photographs
    DROP CONSTRAINT IF EXISTS photographs_context_check;

ALTER TABLE public.photographs
    ALTER COLUMN photograph_context DROP DEFAULT;

ALTER TABLE public.photographs
    ALTER COLUMN photograph_context TYPE public.photograph_context
    USING (CASE WHEN photograph_context = 1 THEN 'post' ELSE 'photography' END)::public.photograph_context;

ALTER TABLE public.photographs
    ALTER COLUMN photograph_context SET DEFAULT 'photography',
    ALTER COLUMN photograph_context SET NOT NULL;

UPDATE public.photographs
SET photograph_context = 'photography'
WHERE photograph_context IS NULL;

CREATE INDEX photographs_context_idx ON public.photographs USING btree (photograph_context);
