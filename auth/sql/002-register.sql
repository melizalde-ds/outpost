CREATE OR REPLACE FUNCTION public.register(p_username text, p_password_hash text)
RETURNS uuid
LANGUAGE sql
SET search_path = pg_catalog, public
AS $$
    INSERT INTO public.auth (username, password_hash)
    VALUES (p_username, p_password_hash)
    RETURNING user_id;
$$;