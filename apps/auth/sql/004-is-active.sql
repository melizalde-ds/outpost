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
