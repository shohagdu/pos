-- 0122 cross-product Buy X Get Y on returns (Final_docs/05 §8): when the
-- customer returns the items that earned a free item but keeps the free
-- one, the free item is charged at its normal price (taken off the refund);
-- returning the free item later gives that back. `promo_charge` is that
-- adjustment on a return line (+ charged, − given back), so the line's own
-- value (`line_total + promo_charge`) still splits the sale line exactly.

ALTER TABLE sale_return_items
    ADD COLUMN promo_charge              DECIMAL(18,2) NOT NULL DEFAULT 0 AFTER line_total,
    ADD COLUMN promo_charge_promotion_id BINARY(16)    NULL AFTER promo_charge;
