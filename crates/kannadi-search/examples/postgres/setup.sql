-- Kannada full-text search in Postgres. See README.md in this folder.
-- Needs kannada.rules in $SHAREDIR/tsearch_data/ (pg_config --sharedir).

CREATE EXTENSION IF NOT EXISTS unaccent;

-- Spelling variants: joiners, the nukta, Old Kannada letters, 2-part vowel signs,
-- a nasal with virama for the anusvara, and Kannada digits.
CREATE TEXT SEARCH DICTIONARY kannada_unaccent (TEMPLATE = unaccent, RULES = 'kannada');

CREATE TEXT SEARCH CONFIGURATION kannada (COPY = simple);
ALTER TEXT SEARCH CONFIGURATION kannada
    ALTER MAPPING FOR word, asciiword, numword, hword, hword_part, hword_asciipart,
        hword_numpart, asciihword, numhword
    WITH kannada_unaccent, simple;

CREATE TABLE documents (
    id integer PRIMARY KEY,
    body text NOT NULL,
    -- Option 1: spelling variants folded inside Postgres.
    body_search tsvector GENERATED ALWAYS AS (to_tsvector('kannada', body)) STORED,
    -- Option 2: kannadi-search keys (spelling, endings and legacy text), written by the
    -- application as array_to_tsvector(ARRAY[...keys]).
    body_keys tsvector
);
CREATE INDEX documents_body_search ON documents USING gin (body_search);
CREATE INDEX documents_body_keys ON documents USING gin (body_keys);
