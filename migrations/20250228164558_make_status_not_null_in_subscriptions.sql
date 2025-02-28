BEGIN;
    -- Backfill
    UPDATE subscriptions
        SET status = 'confirmed'
        WHERE status IS NULL;
    -- Make status NOT NULL
    ALTER TABLE subscriptions
        ALTER COLUMN status SET NOT NULL;
COMMIT;
