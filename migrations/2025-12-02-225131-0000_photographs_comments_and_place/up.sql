ALTER TABLE public.photographs
 ADD COLUMN photograph_comments varchar NOT NULL,
 ADD COLUMN photograph_lat double precision NOT NULL,
 ADD COLUMN photograph_lon double precision NOT NULL;

CREATE INDEX photographs_lat_idx ON public.photographs USING btree (photograph_lat);
CREATE INDEX photographs_lon_idx ON public.photographs USING btree (photograph_lon);
