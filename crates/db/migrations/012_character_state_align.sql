-- Align character_state columns with application code and frontend types.
-- The canonical 001 schema used money/wanted/extra, but the data-access layer
-- and CharacterState frontend type expect resources/current_status/emotion.
ALTER TABLE character_state ADD COLUMN IF NOT EXISTS resources VARCHAR;
ALTER TABLE character_state ADD COLUMN IF NOT EXISTS current_status VARCHAR;
ALTER TABLE character_state ADD COLUMN IF NOT EXISTS emotion VARCHAR;
