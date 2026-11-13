-- 0118 POS product panel switch.
-- Shops that sell mostly by barcode scan can hide the product grid on the POS
-- screen so the cart takes the full width (Settings → Invoice & POS).

ALTER TABLE tenant_settings
    ADD COLUMN show_product_panel BOOLEAN NOT NULL DEFAULT TRUE AFTER auto_print_receipt;
