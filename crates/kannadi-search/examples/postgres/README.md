# Kannada search in Postgres

2 setups, which can live side by side. [setup.sql](setup.sql) creates both, and [demo.sql](demo.sql) adds rows and queries. Both were run on Postgres 19.0 (the `postgres:19.0` Docker image, `C.UTF-8`), whose default text search parser keeps Kannada words whole, joiners included.

## 1. Spelling variants, inside Postgres

The `unaccent` extension replaces strings by a rules file. [kannada.rules](kannada.rules) folds the spelling variants that need no context: joiners and other invisible characters, the nukta, Old Kannada letters, vowel signs typed in 2 parts, a nasal with virama before a consonant of its group, and Kannada digits. The `kannada` text search configuration runs it before the `simple` dictionary.

```sh
cp kannada.rules "$(pg_config --sharedir)/tsearch_data/"
psql -f setup.sql
```

```sql
SELECT id FROM documents WHERE body_search @@ plainto_tsquery('kannada', 'ಕನ್ನಡ');
```

This finds ಕನ್‌ನಡ for ಕನ್ನಡ and ೨೦೨೬ for 2026. It does not remove endings, read legacy text, or apply the rules that depend on context, such as ೦ typed for ಂ after a letter.

The rules file is generated: `cargo run -p kannadi-search --example postgres_rules > kannada.rules`, and a test checks that the committed file matches.

## 2. kannadi-search keys, from the application

For endings and legacy text, the application runs kannadi-search and stores the keys as a `tsvector`, with `array_to_tsvector`, which takes the keys as they are. Queries take the keys of the query the same way.

```sh
cargo run -p kannadi-search --example postgres_keys < documents.txt
# array_to_tsvector(ARRAY['ಮರ', 'ಹಕ್ಕಿ']::text[])
```

```sql
UPDATE documents SET body_keys = array_to_tsvector(ARRAY['ಮರ', 'ಹಕ್ಕಿ']::text[]) WHERE id = 2;
SELECT id FROM documents WHERE body_keys @@ 'ಮರ'::tsquery;
```

In demo.sql, ಮರ finds ಮರಗಳಲ್ಲಿ, and ಕನ್ನಡ finds the legacy line PÀ£ÀßqÀ ¨sÁµÉ.

## Run the demo

```sh
docker run -d --name kannada-pg -e POSTGRES_PASSWORD=pg \
  -v "$PWD/kannada.rules:/usr/share/postgresql/19/tsearch_data/kannada.rules:ro" \
  -v "$PWD:/demo:ro" -w /demo postgres:19.0
until docker exec kannada-pg pg_isready -U postgres; do sleep 1; done
docker exec -w /demo kannada-pg psql -U postgres -f demo.sql
```
