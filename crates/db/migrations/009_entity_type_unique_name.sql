-- Add UNIQUE constraint to entity_type.name for idempotent seeding
-- This allows INSERT ... ON CONFLICT (name) DO NOTHING

-- First, remove duplicates if any exist (keep the first one)
DELETE FROM entity_type a USING entity_type b
WHERE a.name = b.name AND a.id > b.id;

-- Add unique constraint
ALTER TABLE entity_type ADD CONSTRAINT IF NOT EXISTS uq_entity_type_name UNIQUE (name);
