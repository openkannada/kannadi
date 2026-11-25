-- Rows and queries for setup.sql. The body_keys values are what
-- `cargo run -p kannadi-search --example postgres_keys` prints for each body.
\i setup.sql

INSERT INTO documents (id, body, body_keys) VALUES
    (1, 'ಕನ್‌ನಡ ಭಾಷೆ', array_to_tsvector(ARRAY['ಕನ್ನಡ', 'ಭಾಷೆ'])),
    (2, 'ಮರಗಳಲ್ಲಿ ಹಕ್ಕಿಗಳಿವೆ', array_to_tsvector(ARRAY['ಮರ', 'ಹಕ್ಕಿ'])),
    (3, 'ಸಮ್ಬನ್ಧ ೨೦೨೬', array_to_tsvector(ARRAY['ಸಂಬಂಧ', '2026'])),
    (4, 'PÀ£ÀßqÀ ¨sÁµÉ', array_to_tsvector(ARRAY['ಕನ್ನಡ', 'ಭಾಷೆ']));

-- Option 1 finds spelling variants: 1 (ಕನ್‌ನಡ) for ಕನ್ನಡ, 3 for ಸಂಬಂಧ and for 2026.
SELECT 'ಕನ್ನಡ' AS query, array_agg(id ORDER BY id) FROM documents WHERE body_search @@ plainto_tsquery('kannada', 'ಕನ್ನಡ')
UNION ALL
SELECT 'ಸಂಬಂಧ', array_agg(id ORDER BY id) FROM documents WHERE body_search @@ plainto_tsquery('kannada', 'ಸಂಬಂಧ')
UNION ALL
SELECT '2026', array_agg(id ORDER BY id) FROM documents WHERE body_search @@ plainto_tsquery('kannada', '೨೦೨೬');

-- Option 2 also finds endings and legacy text: 2 for ಮರ, 1 and 4 for ಕನ್ನಡ.
SELECT 'ಮರ' AS query, array_agg(id ORDER BY id) FROM documents WHERE body_keys @@ 'ಮರ'::tsquery
UNION ALL
SELECT 'ಕನ್ನಡ', array_agg(id ORDER BY id) FROM documents WHERE body_keys @@ 'ಕನ್ನಡ'::tsquery;
