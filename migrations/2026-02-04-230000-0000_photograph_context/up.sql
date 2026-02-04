ALTER TABLE public.photographs
    ADD COLUMN photograph_context int2 NOT NULL DEFAULT 0;

ALTER TABLE public.photographs
    ADD CONSTRAINT photographs_context_check
    CHECK (photograph_context IN (0, 1));

CREATE INDEX photographs_context_idx ON public.photographs USING btree (photograph_context);
