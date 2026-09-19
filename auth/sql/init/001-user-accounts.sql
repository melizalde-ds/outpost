CREATE TABLE user_accounts(
    user_id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    email varchar(64) NOT NULL UNIQUE,
    display_name text UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);