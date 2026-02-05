CREATE TABLE wasm_module (
    wasm_module_id UUID PRIMARY KEY DEFAULT uuidv7() NOT NULL,
    user_id UUID REFERENCES "users"(user_id) NOT NULL,
    wasm_module_link TEXT NOT NULL,
    wasm_module_description TEXT NOT NULL,
    wasm_module_created_at TIMESTAMPTZ DEFAULT now() NOT NULL,
    wasm_module_updated_at TIMESTAMPTZ DEFAULT now() NOT NULL,
    wasm_module_thumbnail_link TEXT NOT NULL,
    wasm_module_title TEXT NOT NULL
);

CREATE INDEX wasm_module_user_id_idx ON wasm_module (user_id);
CREATE INDEX wasm_module_created_at_idx ON wasm_module (wasm_module_created_at);
CREATE INDEX wasm_module_updated_at_idx ON wasm_module (wasm_module_updated_at);
