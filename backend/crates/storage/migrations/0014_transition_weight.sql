-- Relative likelihood of a transition in a random walk (ADR 0017). NULL is the default weight 1.
ALTER TABLE transitions ADD COLUMN weight integer CHECK (weight IS NULL OR weight BETWEEN 0 AND 1000);
