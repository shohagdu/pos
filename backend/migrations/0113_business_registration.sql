-- 0113 registration details for businesses outside Bangladesh. `country_code`
-- (already on `tenants`) picks which ones apply and how they are labelled:
-- in Italy `bin_number` holds the Partita IVA, `tax_code` the codice fiscale,
-- and `rea_number` / `registry_office` the R.E.A. entry at the Registro
-- Imprese. Receipts print them under the business name.

ALTER TABLE tenants
    ADD COLUMN tax_code        VARCHAR(16)  AFTER trade_license_no,
    ADD COLUMN rea_number      VARCHAR(20)  AFTER tax_code,
    ADD COLUMN registry_office VARCHAR(100) AFTER rea_number;
