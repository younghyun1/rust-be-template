CREATE TABLE public.photographs (
	photograph_id uuid DEFAULT uuidv7() NOT NULL,
	user_id uuid NOT NULL,
	photograph_shot_at timestamptz NULL,
	photograph_created_at timestamptz DEFAULT now() NOT NULL,
	photograph_updated_at timestamptz DEFAULT now() NOT NULL,
	photograph_image_type int4 NOT NULL,
	photograph_is_on_cloud bool NOT NULL,
	photograph_link varchar NOT NULL,
	CONSTRAINT photographs_pk PRIMARY KEY (photograph_id),
	CONSTRAINT photographs_user_profile_picture_image_types_fk FOREIGN KEY (photograph_image_type) REFERENCES public.user_profile_picture_image_types(image_type_id),
	CONSTRAINT photographs_users_fk FOREIGN KEY (user_id) REFERENCES public.users(user_id) ON DELETE CASCADE ON UPDATE CASCADE
);
CREATE INDEX photographs_photograph_created_at_idx ON public.photographs USING btree (photograph_created_at DESC);
CREATE INDEX photographs_photograph_image_type_idx ON public.photographs USING btree (photograph_image_type);
CREATE INDEX photographs_photograph_is_on_cloud_idx ON public.photographs USING btree (photograph_is_on_cloud);
CREATE INDEX photographs_photograph_shot_at_idx ON public.photographs USING btree (photograph_shot_at DESC);
CREATE INDEX photographs_photograph_updated_at_idx ON public.photographs USING btree (photograph_updated_at DESC);
CREATE INDEX photographs_user_id_idx ON public.photographs USING btree (user_id);
