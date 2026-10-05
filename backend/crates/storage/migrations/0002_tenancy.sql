-- Tenant separation. See docs/specification/08-usermanagement.md and
-- docs/adr/0005-jwt-auth-and-tenancy.md.
--
-- The tenant lives on the project only: every other entity is reachable just
-- through a project, so one column expresses the whole boundary. The tenant id
-- is whatever the identity provider puts in the configured claim, so it is
-- text rather than a uuid.

ALTER TABLE projects ADD COLUMN tenant_id text NOT NULL DEFAULT 'dev';
-- The default exists only to backfill rows created before tenancy; new rows
-- must state their tenant.
ALTER TABLE projects ALTER COLUMN tenant_id DROP DEFAULT;

CREATE INDEX projects_tenant_idx ON projects (tenant_id, created_at, id);

-- A job is not reachable through a project, so it carries the tenant that
-- started it. Null means "created before tenancy" and is visible to no tenant.
ALTER TABLE jobs ADD COLUMN tenant_id text;
CREATE INDEX jobs_tenant_idx ON jobs (tenant_id);
