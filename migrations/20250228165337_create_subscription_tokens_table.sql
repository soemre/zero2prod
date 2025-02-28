CREATE TABLE subscription_tokens (
    token TEXT NOT NULL,
    subscriber_id uuid NOT NULL
        REFERENCES subscriptions (id),
        PRIMARY KEY(token)
);
