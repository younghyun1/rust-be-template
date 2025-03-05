DROP INDEX IF EXISTS idx_phone_prefix;
ALTER TABLE public.iso_country DROP COLUMN IF EXISTS phone_prefix;

CREATE TABLE public.iso_phone_prefix (
    prefix_id int4 NOT NULL,
    country_code int4 NOT NULL,
    phone_prefix varchar(10) NOT NULL,
    CONSTRAINT iso_phone_prefix_pkey PRIMARY KEY (prefix_id),
    CONSTRAINT iso_phone_prefix_country_code_fkey FOREIGN KEY (country_code) REFERENCES public.iso_country (country_code)
);

CREATE UNIQUE INDEX idx_phone_prefix_country ON public.iso_phone_prefix USING btree (country_code, phone_prefix);
