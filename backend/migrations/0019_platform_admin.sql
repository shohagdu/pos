-- 0019 platform admin (DB design §9.15, Requirement Analysis §7.3): platform
-- staff are ordinary users with a staff role. Grant the first owner with
-- `pos-api platform-staff <phone> platform_owner`.

CREATE TABLE platform_staff (
    user_id       BINARY(16)  NOT NULL,
    staff_role    VARCHAR(20) NOT NULL,
    status        VARCHAR(20) NOT NULL DEFAULT 'active',
    mfa_required  BOOLEAN     NOT NULL DEFAULT TRUE,
    created_at    DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    created_by    BINARY(16),
    PRIMARY KEY (user_id),
    CONSTRAINT fk_platform_staff_user   FOREIGN KEY (user_id) REFERENCES users (id),
    CONSTRAINT ck_platform_staff_role   CHECK (staff_role IN ('platform_owner','billing','support')),
    CONSTRAINT ck_platform_staff_status CHECK (status IN ('active','disabled'))
);

-- How owners pay for a subscription manually (SUB-R05, ADM-R05). Edited by
-- the platform owner in the admin console.
INSERT INTO platform_settings (setting_key, value) VALUES (
    'billing.payment_instructions',
    JSON_OBJECT(
        'bkash', 'Send Money to 01XXXXXXXXX (bKash personal) and enter the TrxID.',
        'nagad', 'Send Money to 01XXXXXXXXX (Nagad) and enter the transaction ID.',
        'bank_transfer', 'Deposit to account 0000000000, Bank Name, Branch; enter the deposit reference.',
        'note', 'Your subscription is extended as soon as the payment is verified (usually within a few hours).'
    )
);
