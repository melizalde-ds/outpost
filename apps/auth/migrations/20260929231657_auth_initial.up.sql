CREATE TABLE auth(
    user_id uuid PRIMARY KEY DEFAULT uuidv7(),
    username text NOT NULL UNIQUE,
    password_hash text NOT NULL,
    active boolean NOT NULL DEFAULT true,
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE OR REPLACE FUNCTION public.register(p_username text, p_password_hash text)
RETURNS uuid
LANGUAGE sql
SET search_path = pg_catalog, public
AS $$
    INSERT INTO public.auth (username, password_hash)
    VALUES (p_username, p_password_hash)
    RETURNING user_id;
$$;

CREATE OR REPLACE FUNCTION public.get_credentials(p_username text)
RETURNS TABLE (user_id uuid, password_hash text, active boolean)
LANGUAGE sql
STABLE
SET search_path = pg_catalog, public
AS $$
    SELECT a.user_id, a.password_hash, a.active
    FROM public.auth a
    WHERE a.username = p_username;
$$;

CREATE OR REPLACE FUNCTION public.is_active(p_user_id uuid)
RETURNS boolean
LANGUAGE sql
STABLE
SET search_path = pg_catalog, public
AS $$
    SELECT EXISTS (
        SELECT 1 FROM public.auth
        WHERE user_id = p_user_id AND active
    );
$$;

GRANT SELECT, INSERT ON public.auth TO auth_app;
