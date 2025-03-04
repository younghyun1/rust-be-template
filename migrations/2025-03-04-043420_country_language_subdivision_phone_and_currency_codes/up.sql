-- Table for ISO 639 language codes
CREATE TABLE iso_language (
    language_code INTEGER PRIMARY KEY,
    language_alpha2 CHAR(2) NOT NULL,
    language_alpha3 CHAR(3) NOT NULL,
    language_eng_name VARCHAR(255) NOT NULL
);

CREATE UNIQUE INDEX idx_language_alpha2 ON iso_language(language_alpha2);
CREATE UNIQUE INDEX idx_language_alpha3 ON iso_language(language_alpha3);

-- Table for ISO 4217 currency codes
CREATE TABLE iso_currency (
    currency_code INTEGER PRIMARY KEY,
    currency_alpha3 CHAR(3) NOT NULL,
    currency_name VARCHAR(255) NOT NULL
);

CREATE UNIQUE INDEX idx_currency_alpha3 ON iso_currency(currency_alpha3);

-- Table for ISO 3166 country codes and their associated primary language and currency
CREATE TABLE iso_country (
    country_code INTEGER PRIMARY KEY,
    country_alpha2 CHAR(2) NOT NULL,
    country_alpha3 CHAR(3) NOT NULL,
    country_eng_name VARCHAR(255) NOT NULL,
    country_primary_language INTEGER,
    country_currency INTEGER,
    FOREIGN KEY (country_primary_language) REFERENCES iso_language(language_code),
    FOREIGN KEY (country_currency) REFERENCES iso_currency(currency_code)
);

CREATE UNIQUE INDEX idx_country_alpha2 ON iso_country(country_alpha2);
CREATE UNIQUE INDEX idx_country_alpha3 ON iso_country(country_alpha3);

-- Table for international phone prefixes (refer to E.164 standard)
CREATE TABLE iso_phone_prefix (
    prefix_id INTEGER PRIMARY KEY,
    country_code INTEGER NOT NULL,
    phone_prefix VARCHAR(10) NOT NULL,
    FOREIGN KEY (country_code) REFERENCES iso_country(country_code)
);

CREATE UNIQUE INDEX idx_phone_prefix_country ON iso_phone_prefix(country_code, phone_prefix);

-- Table for ISO 3166-2 country subdivisions
CREATE TABLE iso_country_subdivision (
    subdivision_id INTEGER PRIMARY KEY,
    country_code INTEGER NOT NULL,
    subdivision_code VARCHAR(10) NOT NULL,
    subdivision_name VARCHAR(255) NOT NULL,
    subdivision_type VARCHAR(50),
    FOREIGN KEY (country_code) REFERENCES iso_country(country_code),
    UNIQUE (country_code, subdivision_code)
);

CREATE INDEX idx_subdivision_country ON iso_country_subdivision(country_code);

ALTER TABLE public.users
    ADD COLUMN user_country INTEGER NOT NULL,
    ADD COLUMN user_language INTEGER NOT NULL,
    ADD COLUMN user_subdivision INTEGER,
    ADD CONSTRAINT fk_user_country FOREIGN KEY (user_country) REFERENCES iso_country(country_code),
    ADD CONSTRAINT fk_user_language FOREIGN KEY (user_language) REFERENCES iso_language(language_code),
    ADD CONSTRAINT fk_user_subdivision FOREIGN KEY (user_subdivision) REFERENCES iso_country_subdivision(subdivision_id);

CREATE INDEX idx_users_country ON public.users(user_country);
CREATE INDEX idx_users_language ON public.users(user_language);
CREATE INDEX idx_users_subdivision ON public.users(user_subdivision);
