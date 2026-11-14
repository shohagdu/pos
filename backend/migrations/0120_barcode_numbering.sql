-- 0120 Barcode numbering from settings.
-- Automatic barcodes follow the business's own numbering, like SKUs: a prefix
-- plus a zero-padded running number (`200000001`), set under Settings →
-- Product codes. They were fixed in-store EAN-13s (`20` + 10 digits + check
-- digit) before; those stay on their products and keep scanning.

-- DROP CONSTRAINT (not DROP CHECK) so it also runs on MariaDB.
ALTER TABLE document_sequences DROP CONSTRAINT ck_docseq_type;
ALTER TABLE document_sequences ADD CONSTRAINT ck_docseq_type CHECK (doc_type IN ('sale','sale_return','purchase',
    'purchase_return','goods_receipt','payment_in','payment_out','expense','income','adjustment','transfer','count',
    'journal','customer','supplier','product','barcode'));

-- One tenant-wide barcode sequence for every business that numbers products.
-- `20` + 7 digits stays clear of the 8/12/13-digit retail formats.
SET @app_platform = 1;
INSERT INTO document_sequences (tenant_id, id, branch_id, doc_type, prefix, include_period, padding)
SELECT s.tenant_id, UNHEX(REPLACE(UUID(), '-', '')), NULL, 'barcode', '20', FALSE, 7
FROM document_sequences s
WHERE s.doc_type = 'product' AND s.branch_id IS NULL;
SET @app_platform = NULL;
