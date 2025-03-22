CREATE TABLE public.user_profile_picture_image_types (
    image_type_id int4 NOT NULL,
    image_type_name varchar NOT NULL,
    CONSTRAINT u_p_p_image_types_pkey PRIMARY KEY (image_type_id)
);

INSERT INTO
    public.user_profile_picture_image_types (image_type_id, image_type_name)
VALUES
    (1, 'jpg'),
    (2, 'png'),
    (3, 'tiff'),
    (4, 'avif'),
    (5, 'webp'),
    (6, 'gif');

CREATE TABLE public.user_profile_pictures (
    user_profile_picture_id uuid DEFAULT uuid_generate_v4 () NOT NULL,
    user_id uuid NOT NULL,
    user_profile_picture_created_at timestamptz DEFAULT now () NOT NULL,
    user_profile_picture_updated_at timestamptz DEFAULT now () NOT NULL,
    user_profile_picture_image_type int4 NOT NULL,
    user_profile_picture_is_on_cloud bool NOT NULL,
    user_profile_picture_link varchar,
    CONSTRAINT user_profile_pictures_pkey PRIMARY KEY (user_profile_picture_id),
    CONSTRAINT fk_user_profile_pictures_user FOREIGN KEY (user_id) REFERENCES public.users (user_id),
    CONSTRAINT fk_user_profile_pictures_image_type FOREIGN KEY (user_profile_picture_image_type) REFERENCES public.user_profile_picture_image_types (image_type_id)
);

CREATE INDEX idx_user_profile_picture_user_id ON public.user_profile_pictures (user_id);

CREATE INDEX idx_user_profile_picture_image_type ON public.user_profile_pictures (user_profile_picture_image_type);

CREATE INDEX idx_user_profile_picture_created_at ON public.user_profile_pictures (user_profile_picture_created_at);

CREATE INDEX idx_user_profile_picture_updated_at ON public.user_profile_pictures (user_profile_picture_updated_at);

CREATE INDEX idx_user_profile_picture_is_on_cloud ON public.user_profile_pictures (user_profile_picture_is_on_cloud);
