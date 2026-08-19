-- Add content_hash to approval_record for version binding
-- This ensures approval is tied to exact proposal version

ALTER TABLE approval_record ADD COLUMN IF NOT EXISTS content_hash VARCHAR;

-- Create index for quick lookups
CREATE INDEX IF NOT EXISTS idx_approval_content_hash ON approval_record(content_hash);
