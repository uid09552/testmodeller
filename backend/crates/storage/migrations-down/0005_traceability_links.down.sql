-- Reverses migrations/0005_traceability_links.sql. Run by hand; sqlx does not.
-- Puts the links back into the description as labelled lines, ahead of any
-- existing text, then drops the columns.
UPDATE test_cases
SET description = nullif(concat_ws(E'\n',
        CASE WHEN implementation_url IS NOT NULL THEN 'Implementation: ' || implementation_url END,
        CASE WHEN backlog_url IS NOT NULL THEN 'Backlog: ' || backlog_url END,
        description), '')
WHERE implementation_url IS NOT NULL OR backlog_url IS NOT NULL;

DROP INDEX test_cases_backlog_key_idx;
ALTER TABLE test_cases
    DROP COLUMN backlog_key,
    DROP COLUMN backlog_url,
    DROP COLUMN implementation_url;
DELETE FROM _sqlx_migrations WHERE version = 5;
