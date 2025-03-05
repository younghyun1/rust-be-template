DROP TABLE iso_phone_prefix;

ALTER TABLE public.iso_country ADD COLUMN phone_prefix varchar(10);
CREATE INDEX idx_phone_prefix ON public.iso_country USING btree (phone_prefix);