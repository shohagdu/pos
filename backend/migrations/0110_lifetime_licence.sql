-- 0110 one-time (lifetime) licences. A business can be sold a licence once,
-- with no end date: `billing_cycle = 'lifetime'` and `current_period_end`
-- NULL keep the subscription active for good (pos_billing::status::effective).
-- Plans may carry a one-time price beside the monthly/quarterly/annual ones.

ALTER TABLE subscriptions
    DROP CHECK ck_subscriptions_cycle,
    ADD CONSTRAINT ck_subscriptions_cycle CHECK (billing_cycle IN ('monthly','quarterly','annual','lifetime','custom'));

ALTER TABLE plan_prices
    DROP CHECK ck_plan_prices_cycle,
    ADD CONSTRAINT ck_plan_prices_cycle CHECK (billing_cycle IN ('monthly','quarterly','annual','lifetime'));
