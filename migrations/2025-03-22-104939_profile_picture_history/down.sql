DROP INDEX IF EXISTS idx_user_profile_picture_updated_at;

DROP INDEX IF EXISTS idx_user_profile_picture_created_at;

DROP INDEX IF EXISTS idx_user_profile_picture_image_type;

DROP INDEX IF EXISTS idx_user_profile_picture_user_id;

DROP TABLE IF EXISTS public.user_profile_pictures;

DROP TABLE IF EXISTS public.user_profile_picture_image_types;