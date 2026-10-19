-- 0008 login lockout lookup (AUTH-R03): failures per identifier + IP in the
-- last 15 minutes are counted on every login attempt.
CREATE INDEX idx_auth_events_identifier ON auth_events (identifier, event, created_at);
