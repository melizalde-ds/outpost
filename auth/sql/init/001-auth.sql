CREATE TABLE auth(
    user_id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    username text NOT NULL UNIQUE,
    password text NOT NULL,
    updated_at timestamptz NOT NULL DEFAULT now()
);