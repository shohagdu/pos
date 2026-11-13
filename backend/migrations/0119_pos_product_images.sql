-- 0119 POS product images switch.
-- Shops without product photos (or on slow tablets) can hide the thumbnails
-- in the POS product panel so each card is smaller (Settings → Invoice, tax & POS).

ALTER TABLE tenant_settings
    ADD COLUMN show_product_images BOOLEAN NOT NULL DEFAULT TRUE AFTER show_product_panel;
