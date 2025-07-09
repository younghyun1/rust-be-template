CREATE TABLE visitation_data (
    visitation_data_id BIGSERIAL PRIMARY KEY,
    latitude DOUBLE PRECISION NOT NULL,
    longitude DOUBLE PRECISION NOT NULL,
    ip_address INET NOT NULL,
    city VARCHAR NOT NULL,
    country VARCHAR NOT NULL,
    visited_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_visitation_data_city ON visitation_data(city);
CREATE INDEX idx_visitation_data_country ON visitation_data(country);

CREATE INDEX idx_visitation_data_visited_at ON visitation_data(visited_at);
