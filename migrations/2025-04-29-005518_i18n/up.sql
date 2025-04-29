CREATE TABLE public.i18n_strings (
	i18n_string_id uuid DEFAULT uuid_generate_v4() NOT NULL,
	i18n_string_content varchar NOT NULL,
	i18n_string_created_at timestamptz DEFAULT now() NOT NULL,
	i18n_string_created_by uuid DEFAULT '00000000-0000-0000-0000-000000000000'::uuid NOT NULL,
	i18n_string_updated_at timestamptz DEFAULT now() NOT NULL,
	i18n_string_updated_by uuid NOT NULL,
	i18n_string_language_code int4 NOT NULL,
	i18n_string_country_code int4 NOT NULL,
	i18n_string_country_subdivision_code varchar NULL,
	i18n_string_reference_key varchar NOT NULL,
	CONSTRAINT i18n_strings_pk PRIMARY KEY (i18n_string_id),
	CONSTRAINT i18n_strings_unique UNIQUE (i18n_string_reference_key, i18n_string_country_subdivision_code, i18n_string_country_code, i18n_string_language_code),
	CONSTRAINT i18n_strings_iso_country_fk FOREIGN KEY (i18n_string_country_code) REFERENCES public.iso_country(country_code),
	CONSTRAINT i18n_strings_iso_country_subdivision_fk FOREIGN KEY (i18n_string_country_code,i18n_string_country_subdivision_code) REFERENCES public.iso_country_subdivision(country_code,subdivision_code),
	CONSTRAINT i18n_strings_iso_language_fk FOREIGN KEY (i18n_string_language_code) REFERENCES public.iso_language(language_code),
	CONSTRAINT i18n_strings_users_fk FOREIGN KEY (i18n_string_created_by) REFERENCES public.users(user_id) ON DELETE SET DEFAULT ON UPDATE CASCADE,
	CONSTRAINT i18n_strings_users_updated_by_fk FOREIGN KEY (i18n_string_updated_by) REFERENCES public.users(user_id) ON DELETE SET DEFAULT ON UPDATE CASCADE
);
CREATE INDEX i18n_strings_i18n_string_country_code_idx ON public.i18n_strings USING btree (i18n_string_country_code);
CREATE INDEX i18n_strings_i18n_string_country_subdivision_code_idx ON public.i18n_strings USING btree (i18n_string_country_subdivision_code);
CREATE INDEX i18n_strings_i18n_string_created_at_idx ON public.i18n_strings USING btree (i18n_string_created_at DESC);
CREATE INDEX i18n_strings_i18n_string_created_by_idx ON public.i18n_strings USING btree (i18n_string_created_by);
CREATE INDEX i18n_strings_i18n_string_language_idx ON public.i18n_strings USING btree (i18n_string_language_code);
CREATE INDEX i18n_strings_i18n_string_reference_key_idx ON public.i18n_strings USING btree (i18n_string_reference_key);
CREATE INDEX i18n_strings_i18n_string_updated_at_idx ON public.i18n_strings USING btree (i18n_string_updated_at DESC);
CREATE INDEX i18n_strings_i18n_string_updated_by_idx ON public.i18n_strings USING btree (i18n_string_updated_by);
