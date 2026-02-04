DROP INDEX IF EXISTS photographs_context_idx;

ALTER TABLE public.photographs
    ALTER COLUMN photograph_context TYPE int2
    USING (CASE WHEN photograph_context = 'post' THEN 1 ELSE 0 END)::int2;

ALTER TABLE public.photographs
    ALTER COLUMN photograph_context SET DEFAULT 0,
    ALTER COLUMN photograph_context SET NOT NULL;

ALTER TABLE public.photographs
    ADD CONSTRAINT photographs_context_check
    CHECK (photograph_context IN (0, 1));

CREATE INDEX photographs_context_idx ON public.photographs USING btree (photograph_context);

DROP TYPE IF EXISTS public.photograph_context;
