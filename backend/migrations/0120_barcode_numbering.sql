-- 0120 Barcode numbering from settings.
-- Automatic barcodes follow the business's own numbering, like SKUs: a prefix
-- plus a zero-padded running number (`200000001`), set under Settings →
-- Product codes. They were fixed in-store EAN-13s (`20` + 10 digits + check
-- digit) before; those stay on their products and keep scanning.
--
-- Runs on MySQL 8 and MariaDB 10.5, and is safe to run again after a partial
-- run (MariaDB has no `DROP CHECK`, MySQL has no `DROP CONSTRAINT IF EXISTS`,
-- so the old check is dropped only when it is there).

SET @drop_ck_docseq_type = (
    SELECT IF(COUNT(*) > 0, 'ALTER TABLE document_sequences DROP CONSTRAINT ck_docseq_type', 'DO 0')
    FROM information_schema.TABLE_CONSTRAINTS
    WHERE CONSTRAINT_SCHEMA = DATABASE() AND TABLE_NAME = 'document_sequences'
      AND CONSTRAINT_NAME = 'ck_docseq_type' AND CONSTRAINT_TYPE = 'CHECK'
);
PREPARE drop_ck_docseq_type FROM @drop_ck_docseq_type;
EXECUTE drop_ck_docseq_type;
DEALLOCATE PREPARE drop_ck_docseq_type;
SET @drop_ck_docseq_type = NULL;

ALTER TABLE document_sequences ADD CONSTRAINT ck_docseq_type CHECK (doc_type IN ('sale','sale_return','purchase',
    'purchase_return','goods_receipt','payment_in','payment_out','expense','income','adjustment','transfer','count',
    'journal','customer','supplier','product','barcode'));

-- One tenant-wide barcode sequence for every business that numbers products.
-- `20` + 7 digits stays clear of the 8/12/13-digit retail formats. Businesses
-- that already have one (an earlier, partial run) are left alone.
SET @app_platform = 1;
INSERT INTO document_sequences (tenant_id, id, branch_id, doc_type, prefix, include_period, padding)
SELECT s.tenant_id, UNHEX(REPLACE(UUID(), '-', '')), NULL, 'barcode', '20', FALSE, 7
FROM document_sequences s
WHERE s.doc_type = 'product' AND s.branch_id IS NULL
  AND NOT EXISTS (SELECT 1 FROM document_sequences b
                  WHERE b.tenant_id = s.tenant_id AND b.doc_type = 'barcode' AND b.branch_id IS NULL);
SET @app_platform = NULL;
