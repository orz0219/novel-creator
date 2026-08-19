-- Outbox pattern for reliable event delivery
-- Events are written to outbox in the same transaction as the mutation
-- A separate worker reads and delivers events

CREATE TABLE IF NOT EXISTS event_outbox (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    event_type VARCHAR NOT NULL,
    aggregate_type VARCHAR NOT NULL,
    aggregate_id VARCHAR NOT NULL,
    payload JSONB NOT NULL,
    status VARCHAR NOT NULL DEFAULT 'Pending', -- Pending, Delivered, Failed
    retry_count INTEGER NOT NULL DEFAULT 0,
    max_retries INTEGER NOT NULL DEFAULT 3,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    delivered_at TIMESTAMPTZ,
    error_message TEXT
);

CREATE INDEX IF NOT EXISTS idx_outbox_status ON event_outbox(status);
CREATE INDEX IF NOT EXISTS idx_outbox_project ON event_outbox(project_id);
CREATE INDEX IF NOT EXISTS idx_outbox_created ON event_outbox(created_at);
