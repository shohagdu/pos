-- 0115 product warranty.
-- A product may carry a warranty of N days, months or years (both columns
-- NULL = no warranty). A sale line copies the product's warranty when it is
-- sold and stores the date it runs out (business date + tenure), so later
-- edits to the product do not change warranties already given.

ALTER TABLE products
    ADD COLUMN warranty_period SMALLINT,
    ADD COLUMN warranty_unit   VARCHAR(10),
    ADD CONSTRAINT ck_products_warranty CHECK (
        (warranty_period IS NULL AND warranty_unit IS NULL)
        OR (warranty_period > 0 AND warranty_unit IN ('day','month','year'))
    );

ALTER TABLE sale_items
    ADD COLUMN warranty_period     SMALLINT,
    ADD COLUMN warranty_unit       VARCHAR(10),
    ADD COLUMN warranty_expires_on DATE;
