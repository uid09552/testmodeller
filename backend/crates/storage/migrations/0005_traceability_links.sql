-- Implementation and backlog links become fields of a test case (FR-025, FR-026).
-- Earlier versions packed them into `description` as the lines
-- "Implementation: <url>" and "Backlog: <url>"; this moves them out.
-- docs/specification/02-domain-model.md. The reverse is migrations-down/.

ALTER TABLE test_cases
    ADD COLUMN IF NOT EXISTS implementation_url text CHECK (char_length(implementation_url) <= 2048),
    ADD COLUMN IF NOT EXISTS backlog_url        text CHECK (char_length(backlog_url) <= 2048),
    -- Normalised backlog_url, computed on write: groups spellings of one item.
    ADD COLUMN IF NOT EXISTS backlog_key        text;

CREATE INDEX IF NOT EXISTS test_cases_backlog_key_idx ON test_cases (backlog_key) WHERE backlog_key IS NOT NULL;

-- Move the labelled lines. Only a first line with the exact prefix and a valid
-- http(s) URL is moved; anything else stays in the description untouched.
-- Rerunning finds no such lines, so it changes nothing.
WITH lines AS (
    SELECT t.id, u.n, u.l,
           btrim(substr(u.l, 17), E' \t\r') AS impl,
           btrim(substr(u.l, 10), E' \t\r') AS backlog
    FROM test_cases t,
         unnest(string_to_array(t.description, E'\n')) WITH ORDINALITY AS u(l, n)
    WHERE t.description IS NOT NULL
),
picked AS (
    SELECT id,
        (SELECT min(n) FROM lines x WHERE x.id = p.id AND x.l LIKE 'Implementation: %'
            AND x.impl ~* '^https?://([^/?#@]*@)?[^/?#@:]+' AND x.impl !~ '[[:space:][:cntrl:]]'
            AND char_length(x.impl) <= 2048) AS impl_n,
        (SELECT min(n) FROM lines x WHERE x.id = p.id AND x.l LIKE 'Backlog: %'
            AND x.backlog ~* '^https?://([^/?#@]*@)?[^/?#@:]+' AND x.backlog !~ '[[:space:][:cntrl:]]'
            AND char_length(x.backlog) <= 2048) AS backlog_n
    FROM (SELECT DISTINCT id FROM lines) p
),
moved AS (
    SELECT p.id,
           (SELECT impl FROM lines x WHERE x.id = p.id AND x.n = p.impl_n) AS impl,
           (SELECT backlog FROM lines x WHERE x.id = p.id AND x.n = p.backlog_n) AS backlog,
           nullif(btrim(coalesce((
               SELECT string_agg(x.l, E'\n' ORDER BY x.n) FROM lines x
               WHERE x.id = p.id AND x.n IS DISTINCT FROM p.impl_n AND x.n IS DISTINCT FROM p.backlog_n
           ), ''), E' \t\r\n'), '') AS rest
    FROM picked p
    WHERE p.impl_n IS NOT NULL OR p.backlog_n IS NOT NULL
)
UPDATE test_cases t
SET implementation_url = m.impl,
    backlog_url = m.backlog,
    description = m.rest
FROM moved m
WHERE t.id = m.id;

-- Key of the moved backlog links: lower-case scheme and host, no fragment, no
-- trailing slash, query kept. Mirrors tm_domain::links::backlog_key, and a
-- storage test checks that the two agree.
UPDATE test_cases t
SET backlog_key = lower(k.m[1]) || '://'
        || coalesce(substring(k.m[2] from '^.*@'), '') || lower(substring(k.m[2] from '[^@]*$'))
        || rtrim(k.m[3], '/')
        || CASE WHEN coalesce(k.m[4], '') IN ('', '?') THEN '' ELSE k.m[4] END
FROM (
    SELECT id, regexp_match(backlog_url, '^([A-Za-z]+)://([^/?#]*)([^?#]*)(\?[^#]*)?') AS m
    FROM test_cases WHERE backlog_url IS NOT NULL
) k
WHERE t.id = k.id AND k.m IS NOT NULL;
