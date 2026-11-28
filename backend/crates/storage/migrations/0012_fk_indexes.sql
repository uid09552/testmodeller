-- Indexes on foreign key columns that had none. Without them, deleting a
-- state, model, feature or threat model item makes PostgreSQL scan the whole
-- referencing table to cascade or set NULL. Found with a catalog query for
-- single-column foreign keys that no index leads with.
CREATE INDEX IF NOT EXISTS transitions_from_state_idx ON transitions (from_state);
CREATE INDEX IF NOT EXISTS transitions_to_state_idx ON transitions (to_state);
CREATE INDEX IF NOT EXISTS test_cases_generated_from_model_idx ON test_cases (generated_from_model_id);
CREATE INDEX IF NOT EXISTS proposals_feature_idx ON proposals (feature_id);
CREATE INDEX IF NOT EXISTS proposals_model_idx ON proposals (model_id);
CREATE INDEX IF NOT EXISTS tm_components_linked_component_idx ON tm_components (linked_component_id);
CREATE INDEX IF NOT EXISTS tm_components_linked_feature_idx ON tm_components (linked_feature_id);
CREATE INDEX IF NOT EXISTS tm_components_zone_idx ON tm_components (zone_id);
CREATE INDEX IF NOT EXISTS tm_rule_keys_threat_idx ON tm_rule_keys (threat_id);
CREATE INDEX IF NOT EXISTS tm_threat_assets_asset_idx ON tm_threat_assets (asset_id);
CREATE INDEX IF NOT EXISTS tm_control_components_component_idx ON tm_control_components (component_id);
CREATE INDEX IF NOT EXISTS security_test_controls_control_idx ON security_test_controls (control_id);
