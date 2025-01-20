CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE users (
    user_id UUID PRIMARY KEY DEFAULT uuid_generate_v4 (),
    user_name VARCHAR NOT NULL,
    user_email VARCHAR NOT NULL,
    user_created_at TIMESTAMPTZ NOT NULL DEFAULT now (),
    user_updated_at TIMESTAMPTZ NOT NULL DEFAULT now ()
);

CREATE INDEX idx_users_name ON users (user_name);

CREATE INDEX idx_users_email ON users (user_email);

CREATE INDEX idx_users_created_at ON users (user_created_at);

CREATE INDEX idx_users_updated_at ON users (user_updated_at);
