CREATE TABLE public.token_family (
    family_id  uuid PRIMARY KEY DEFAULT uuidv7(),
    user_id    uuid NOT NULL REFERENCES public.auth(user_id),
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX token_family_user_id_idx
    ON public.token_family (user_id);

CREATE TABLE public.token_family_revocation (
    family_id  uuid PRIMARY KEY REFERENCES public.token_family(family_id),
    reason     text NOT NULL CHECK (reason IN ('reuse', 'logout', 'logout_all', 'inactive', 'blocked')),
    revoked_at timestamptz NOT NULL DEFAULT now()
);

CREATE OR REPLACE FUNCTION public.create_token_family(p_user_id uuid)
RETURNS uuid
LANGUAGE sql
SET search_path = pg_catalog, public
AS $$
    INSERT INTO public.token_family (user_id)
    VALUES (p_user_id)
    RETURNING family_id;
$$;

CREATE OR REPLACE FUNCTION public.get_user_token_families(p_user_id uuid)
RETURNS TABLE (family_id uuid)
LANGUAGE sql
STABLE
SET search_path = pg_catalog, public
AS $$
    SELECT f.family_id
    FROM public.token_family f
    WHERE f.user_id = p_user_id
      AND NOT EXISTS (
          SELECT 1 FROM public.token_family_revocation r
          WHERE r.family_id = f.family_id
      );
$$;

CREATE OR REPLACE FUNCTION public.revoke_token_family(p_family_id uuid, p_reason text)
RETURNS int
LANGUAGE sql
SET search_path = pg_catalog, public
AS $$
    WITH ins AS (
        INSERT INTO public.token_family_revocation (family_id, reason)
        VALUES (p_family_id, p_reason)
        ON CONFLICT (family_id) DO NOTHING
        RETURNING 1
    )
    SELECT count(*)::int FROM ins;
$$;

CREATE OR REPLACE FUNCTION public.revoke_user_token_families(p_user_id uuid, p_reason text)
RETURNS TABLE (family_id uuid)
LANGUAGE sql
SET search_path = pg_catalog, public
AS $$
    WITH families AS (
        SELECT f.family_id
        FROM public.token_family f
        WHERE f.user_id = p_user_id
    ), ins AS (
        INSERT INTO public.token_family_revocation (family_id, reason)
        SELECT families.family_id, p_reason
        FROM families
        ON CONFLICT (family_id) DO NOTHING
    )
    SELECT families.family_id FROM families;
$$;

GRANT SELECT, INSERT ON public.token_family, public.token_family_revocation TO auth_app;
