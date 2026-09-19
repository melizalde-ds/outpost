CREATE TABLE auth(
    user_id uuid PRIMARY KEY REFERENCES user_accounts(user_id),
    password text NOT NULL,
    updated_at timestamptz NOT NULL DEFAULT now()
);