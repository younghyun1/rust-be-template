-- To change the role_id type from uuid to serial autoincrementing integer:
-- First, drop any default and constraints referencing the old type
ALTER TABLE public.roles DROP CONSTRAINT IF EXISTS roles_pkey;
ALTER TABLE public.roles DROP CONSTRAINT IF EXISTS roles_role_name_key;

-- get rid of uuidv7 default
ALTER TABLE public.roles
    ALTER COLUMN role_id DROP DEFAULT,
    ALTER COLUMN role_id TYPE integer;

ALTER TABLE public.roles
    ALTER COLUMN role_id SET NOT NULL;

-- Set up auto-increment sequence;
CREATE SEQUENCE IF NOT EXISTS roles_role_id_seq OWNED BY public.roles.role_id;
ALTER TABLE public.roles ALTER COLUMN role_id SET DEFAULT nextval('roles_role_id_seq');

-- Re-create primary and unique key constraints:
ALTER TABLE public.roles
    ADD CONSTRAINT roles_pkey PRIMARY KEY (role_id),
    ADD CONSTRAINT roles_role_name_key UNIQUE (role_name);

INSERT INTO public.roles (role_id, role_name, role_description)
VALUES (0, 'younghyun', 'Administrator role with total access; owner of the site.'),
       (1, 'moderator', 'Moderator role with elevated permissions'),
       (2, 'user', 'Regular user role with limited access'),
       (3, 'guest', 'Guest role with minimal access');
