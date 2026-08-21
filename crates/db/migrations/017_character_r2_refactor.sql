-- ============================================================
-- Character module R2 redesign
-- Idempotent: safe to re-apply against a DB that was already
-- partially migrated (renames are guarded by column existence).
-- ============================================================

-- character_profile: align with new domain struct
DO $$
BEGIN
    -- real_name -> name
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'character_profile' AND column_name = 'real_name'
    ) THEN
        ALTER TABLE character_profile RENAME COLUMN real_name TO name;
    ELSIF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'character_profile' AND column_name = 'name'
    ) THEN
        ALTER TABLE character_profile ADD COLUMN name TEXT;
    END IF;

    -- background -> background_origin
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'character_profile' AND column_name = 'background'
    ) THEN
        ALTER TABLE character_profile RENAME COLUMN background TO background_origin;
    ELSIF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'character_profile' AND column_name = 'background_origin'
    ) THEN
        ALTER TABLE character_profile ADD COLUMN background_origin TEXT;
    END IF;

    -- age -> age_range
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'character_profile' AND column_name = 'age'
    ) THEN
        ALTER TABLE character_profile RENAME COLUMN age TO age_range;
        ALTER TABLE character_profile ALTER COLUMN age_range TYPE TEXT USING age_range::TEXT;
    ELSIF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'character_profile' AND column_name = 'age_range'
    ) THEN
        ALTER TABLE character_profile ADD COLUMN age_range TEXT;
    END IF;

    -- gender already exists (VARCHAR) — keep as-is, just ensure it is present
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'character_profile' AND column_name = 'gender'
    ) THEN
        ALTER TABLE character_profile ADD COLUMN gender TEXT;
    END IF;

    -- legacy columns no longer used by the new model
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'character_profile' AND column_name = 'nickname'
    ) THEN
        ALTER TABLE character_profile DROP COLUMN nickname;
    END IF;
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'character_profile' AND column_name = 'social_status'
    ) THEN
        ALTER TABLE character_profile DROP COLUMN social_status;
    END IF;
END $$;

ALTER TABLE character_profile ADD COLUMN IF NOT EXISTS aliases JSONB NOT NULL DEFAULT '[]'::jsonb;
ALTER TABLE character_profile ADD COLUMN IF NOT EXISTS social_position JSONB;
ALTER TABLE character_profile ADD COLUMN IF NOT EXISTS role_in_story TEXT;
ALTER TABLE character_profile ADD COLUMN IF NOT EXISTS narrative_necessity JSONB;
ALTER TABLE character_profile ADD COLUMN IF NOT EXISTS extra JSONB NOT NULL DEFAULT '{}'::jsonb;

-- character_state: align with new domain struct
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'character_state' AND column_name = 'health'
    ) THEN
        ALTER TABLE character_state RENAME COLUMN health TO physical_state;
    ELSIF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'character_state' AND column_name = 'physical_state'
    ) THEN
        ALTER TABLE character_state ADD COLUMN physical_state TEXT;
    END IF;

    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'character_state' AND column_name = 'cultivation'
    ) THEN
        ALTER TABLE character_state DROP COLUMN cultivation;
    END IF;
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'character_state' AND column_name = 'money'
    ) THEN
        ALTER TABLE character_state DROP COLUMN money;
    END IF;
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'character_state' AND column_name = 'wanted'
    ) THEN
        ALTER TABLE character_state DROP COLUMN wanted;
    END IF;
END $$;

ALTER TABLE character_state ADD COLUMN IF NOT EXISTS mental_state TEXT;
ALTER TABLE character_state ADD COLUMN IF NOT EXISTS resource_state TEXT;
ALTER TABLE character_state ADD COLUMN IF NOT EXISTS social_state TEXT;
ALTER TABLE character_state ADD COLUMN IF NOT EXISTS flags JSONB NOT NULL DEFAULT '[]'::jsonb;

-- character_trait: drop parent_trait_id (flattened)
ALTER TABLE character_trait DROP COLUMN IF EXISTS parent_trait_id;

-- character_goal removed (folded into character_drive); keep character_goal_mind
DROP TABLE IF EXISTS character_goal;

-- New tables (IF NOT EXISTS for idempotency)
CREATE TABLE IF NOT EXISTS character_drive (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_id UUID NOT NULL REFERENCES entity(id),
    primary_goal TEXT,
    motivation TEXT,
    urgency INTEGER NOT NULL DEFAULT 5,
    long_term TEXT,
    current_goal TEXT,
    immediate TEXT,
    hidden_goal TEXT,
    fear TEXT,
    weakness TEXT,
    desire TEXT,
    contradiction TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_char_drive_entity ON character_drive(entity_id);

CREATE TABLE IF NOT EXISTS character_conflict (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_id UUID NOT NULL REFERENCES entity(id),
    conflict_type TEXT,
    description TEXT,
    target_entity_id UUID,
    resolution_status TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_char_conflict_entity ON character_conflict(entity_id);

CREATE TABLE IF NOT EXISTS character_relationship (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_id UUID NOT NULL REFERENCES entity(id),
    target_entity_id UUID,
    relationship_type TEXT,
    attitude TEXT,
    trust_level INTEGER,
    secret_knowledge TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_char_relationship_entity ON character_relationship(entity_id);

CREATE TABLE IF NOT EXISTS character_secret (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_id UUID NOT NULL REFERENCES entity(id),
    content TEXT,
    importance INTEGER,
    reveal_condition TEXT,
    related_entities JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_char_secret_entity ON character_secret(entity_id);

CREATE TABLE IF NOT EXISTS character_capability (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_id UUID NOT NULL REFERENCES entity(id),
    skills JSONB,
    limitations JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_char_capability_entity ON character_capability(entity_id);

CREATE TABLE IF NOT EXISTS character_arc_potential (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_id UUID NOT NULL REFERENCES entity(id),
    starting_state TEXT,
    possible_change TEXT,
    resistance TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_char_arc_entity ON character_arc_potential(entity_id);

CREATE TABLE IF NOT EXISTS character_extension (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_id UUID NOT NULL REFERENCES entity(id),
    extension_type TEXT,
    data JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_char_extension_entity ON character_extension(entity_id);

-- character_memory: add memory_type
ALTER TABLE character_memory ADD COLUMN IF NOT EXISTS memory_type TEXT;
