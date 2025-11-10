-- Reverse the changes made in up.sql

-- Remove inserted data
DELETE FROM public.roles WHERE role_id IN (0, 1, 2);

-- Drop unique and primary key constraints
ALTER TABLE public.roles DROP CONSTRAINT IF EXISTS roles_pkey;
ALTER TABLE public.roles DROP CONSTRAINT IF EXISTS roles_role_name_key;

-- Remove the sequence and default for serial integer
ALTER TABLE public.roles ALTER COLUMN role_id DROP DEFAULT;
DROP SEQUENCE IF EXISTS roles_role_id_seq;

-- Change the type back to uuid
ALTER TABLE public.roles
    ALTER COLUMN role_id TYPE uuid USING role_id::uuid;

-- Remove NOT NULL constraint if it wasn't there before (assume not)
ALTER TABLE public.roles
    ALTER COLUMN role_id DROP NOT NULL;

-- Restore uuidv7 default, if it existed before
ALTER TABLE public.roles ALTER COLUMN role_id SET DEFAULT uuidv7();

-- Restore primary and unique key constraints
ALTER TABLE public.roles
    ADD CONSTRAINT roles_pkey PRIMARY KEY (role_id),
    ADD CONSTRAINT roles_role_name_key UNIQUE (role_name);

-- End of rollback for role_id serial->uuid migration
