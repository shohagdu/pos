-- 0110 one-time (lifetime) licences. A business can be sold a licence once,
-- with no end date: `billing_cycle = 'lifetime'` and `current_period_end`
-- NULL keep the subscription active for good (pos_billing::status::effective).
-- Plans may carry a one-time price beside the monthly/quarterly/annual ones.

-- DROP CONSTRAINT, in its own statement, works on MySQL 8.0.19+ and MariaDB.
ALTER TABLE subscriptions DROP CONSTRAINT ck_subscriptions_cycle;
ALTER TABLE subscriptions
    ADD CONSTRAINT ck_subscriptions_cycle CHECK (billing_cycle IN ('monthly','quarterly','annual','lifetime','custom'));

ALTER TABLE plan_prices DROP CONSTRAINT ck_plan_prices_cycle;
ALTER TABLE plan_prices
    ADD CONSTRAINT ck_plan_prices_cycle CHECK (billing_cycle IN ('monthly','quarterly','annual','lifetime'));
