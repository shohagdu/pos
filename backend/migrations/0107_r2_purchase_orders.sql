-- 0107 purchase orders (PUR-R06, Requirement Analysis §9.2).
--
-- A purchase can now go draft → ordered → approved → (partially_)received:
-- each goods receipt posts the stock it brings, credits the supplier for the
-- value received and journals it. `received_total` is that running value
-- (what the supplier is owed before payments and returns); a bill posted in
-- one step receives everything, so it equals `grand_total`. A partly received
-- order can be closed short: the rest is never received or owed.

ALTER TABLE purchases
    ADD COLUMN received_total DECIMAL(18,2) NOT NULL DEFAULT 0 AFTER grand_total,
    ADD COLUMN ordered_at     DATETIME(6) AFTER expected_date,
    ADD COLUMN ordered_by     BINARY(16)  AFTER ordered_at,
    ADD COLUMN closed_at      DATETIME(6) AFTER posted_by,
    ADD COLUMN closed_by      BINARY(16)  AFTER closed_at;

-- What each receipt line was worth to the supplier (net + VAT + its share of
-- charges) and the VAT in it, so later receipts and reports add up exactly.
ALTER TABLE goods_receipt_items
    ADD COLUMN line_value DECIMAL(18,2) NOT NULL DEFAULT 0,
    ADD COLUMN tax_amount DECIMAL(18,2) NOT NULL DEFAULT 0;

SET @app_platform = 1;

UPDATE purchases SET received_total = grand_total WHERE status = 'received';

UPDATE goods_receipt_items gi
JOIN purchase_items pi ON pi.tenant_id = gi.tenant_id AND pi.id = gi.purchase_item_id
SET gi.line_value = pi.line_total + pi.charges_alloc, gi.tax_amount = pi.tax_amount;

SET @app_platform = NULL;
