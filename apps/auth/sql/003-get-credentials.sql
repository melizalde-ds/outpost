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