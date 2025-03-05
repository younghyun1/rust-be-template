ALTER TABLE public.iso_country ALTER COLUMN country_primary_language DROP NOT NULL;
ALTER TABLE public.iso_country ALTER COLUMN country_currency DROP NOT NULL;
ALTER TABLE public.iso_country ALTER COLUMN phone_prefix DROP NOT NULL;

DELETE FROM iso_country;
