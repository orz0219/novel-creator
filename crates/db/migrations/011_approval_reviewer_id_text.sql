-- Fix a schema drift introduced in the canonical schema:
--   approval_record.reviewer_id was declared as UUID, but the domain model and
--   the DbApprovalPort treat it as a reviewer *username* string
--   (ApprovalRecord.reviewer_id: Option<String>; approve()/reject() bind &str
--   such as "author1"). Rebuilding from the canonical migrations therefore
--   failed every approval with:
--     column "reviewer_id" is of type uuid but expression is of type text
--   Align the column with the code by making it VARCHAR. The column is
--   currently NULL for all rows, so the type cast is safe.
ALTER TABLE approval_record
    ALTER COLUMN reviewer_id TYPE VARCHAR;
