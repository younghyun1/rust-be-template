-- Enable the uuid-ossp extension if it hasn't been enabled yet
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Create Roles Table with appropriate constraints and indexes
CREATE TABLE roles (
    role_id UUID PRIMARY KEY DEFAULT uuid_generate_v4 (),
    role_name TEXT NOT NULL UNIQUE, -- Use TEXT type for strings that do not have a length requirement
    role_description TEXT
);

-- Create an index on role_name for quicker lookups
CREATE INDEX idx_roles_role_name ON roles (role_name);

-- Create Permissions Table with appropriate constraints and indexes
CREATE TABLE permissions (
    permission_id UUID PRIMARY KEY DEFAULT uuid_generate_v4 (),
    permission_name TEXT NOT NULL UNIQUE,
    permission_description TEXT
);

-- Create an index on permission_name for quicker lookups
CREATE INDEX idx_permissions_permission_name ON permissions (permission_name);

-- Create Role_Permissions Join Table with appropriate constraints and indexes
CREATE TABLE role_permissions (
    role_permission_id UUID PRIMARY KEY DEFAULT uuid_generate_v4 (),
    role_id UUID NOT NULL,
    permission_id UUID NOT NULL,
    FOREIGN KEY (role_id) REFERENCES roles (role_id) ON DELETE CASCADE,
    FOREIGN KEY (permission_id) REFERENCES permissions (permission_id) ON DELETE CASCADE,
    UNIQUE (role_id, permission_id)
);

-- Create indexes on role_id and permission_id for optimization of joins
CREATE INDEX idx_role_permissions_role_id ON role_permissions (role_id);

CREATE INDEX idx_role_permissions_permission_id ON role_permissions (permission_id);

-- Create User_Roles Table for Exclusive Roles Assignment with appropriate constraints and indexes
CREATE TABLE user_roles (
    user_role_id UUID PRIMARY KEY DEFAULT uuid_generate_v4 (),
    user_id UUID NOT NULL UNIQUE,
    role_id UUID NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users (user_id) ON DELETE CASCADE,
    FOREIGN KEY (role_id) REFERENCES roles (role_id) ON DELETE CASCADE
);

-- Create indexes on user_id and role_id for optimization of joins
CREATE INDEX idx_user_roles_user_id ON user_roles (user_id);

CREATE INDEX idx_user_roles_role_id ON user_roles (role_id);
