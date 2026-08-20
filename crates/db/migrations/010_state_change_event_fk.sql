-- Fix a schema inconsistency introduced in the canonical schema:
--   state_change.event_id was declared to reference event(id), but the
--   canonical commit path (runtime_ports::commit_changes) persists the
--   DomainEvent into system_events, which carries the entity_id/data/source
--   columns that the `event` table lacks. As a result every commit failed
--   the state_change_event_id_fkey check. Point the FK at the table that
--   actually stores these events.
ALTER TABLE state_change
    DROP CONSTRAINT IF EXISTS state_change_event_id_fkey;

ALTER TABLE state_change
    ADD CONSTRAINT state_change_event_id_fkey
    FOREIGN KEY (event_id) REFERENCES system_events(id);
