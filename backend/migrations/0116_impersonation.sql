-- 0116 owner impersonation.
-- The owner of a business can sign in as one of its staff to see and do what
-- they can. Such a session row names the owner in `impersonator_id`; it is
-- kept across refreshes, ties the session to that one business, and is what
-- lets the owner return to their own account without a password.
-- Audit entries written during it name the owner too.

ALTER TABLE user_sessions
    ADD COLUMN impersonator_id BINARY(16),
    ADD CONSTRAINT fk_user_sessions_impersonator FOREIGN KEY (impersonator_id) REFERENCES users (id) ON DELETE CASCADE;

ALTER TABLE audit_logs
    ADD COLUMN impersonator_id BINARY(16);
