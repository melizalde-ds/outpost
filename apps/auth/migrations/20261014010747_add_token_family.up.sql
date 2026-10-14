CREATE TABLE public.token_family
(
    family_id  uuid PRIMARY KEY     DEFAULT uuidv7(),
    user_id    uuid        NOT NULL
        REFERENCES public.auth (user_id) ON DELETE RESTRICT,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX token_family_user_id_idx
    ON public.token_family (user_id);

CREATE TABLE public.token_family_revocation
(
    family_id  uuid PRIMARY KEY
        REFERENCES public.token_family (family_id) ON DELETE CASCADE,
    reason     text        NOT NULL,
    revoked_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT token_family_revocation_reason_check
        CHECK (reason IN ('reuse', 'logout', 'logout_all', 'inactive', 'blocked', 'erased'))
);

CREATE TABLE public.token_family_window
(
    family_id  uuid PRIMARY KEY
        REFERENCES public.token_family (family_id) ON DELETE CASCADE,
    expires_at timestamptz NOT NULL
);

CREATE FUNCTION public.create_token_family(p_user_id uuid, p_expires_at timestamptz)
    RETURNS uuid
    LANGUAGE sql
    VOLATILE
BEGIN
    ATOMIC
    WITH family AS (
        INSERT INTO public.token_family (user_id)
            VALUES (p_user_id)
            RETURNING family_id)
    INSERT
    INTO public.token_family_window (family_id, expires_at)
    SELECT family.family_id, p_expires_at
    FROM family
    RETURNING token_family_window.family_id;
END;

CREATE FUNCTION public.get_user_token_families(p_user_id uuid)
    RETURNS TABLE
            (
                family_id uuid
            )
    LANGUAGE sql
    STABLE
BEGIN
    ATOMIC
    SELECT f.family_id
    FROM public.token_family f
             JOIN public.token_family_window w ON w.family_id = f.family_id
    WHERE f.user_id = p_user_id
      AND w.expires_at > now()
      AND NOT EXISTS (SELECT 1
                      FROM public.token_family_revocation r
                      WHERE r.family_id = f.family_id);
END;

CREATE FUNCTION public.revoke_token_family(p_family_id uuid, p_reason text)
    RETURNS boolean
    LANGUAGE sql
    VOLATILE
BEGIN
    ATOMIC
    WITH ins AS (
        INSERT INTO public.token_family_revocation (family_id, reason)
            VALUES (p_family_id, p_reason)
            ON CONFLICT (family_id) DO NOTHING
            RETURNING 1)
    SELECT EXISTS (SELECT 1 FROM ins);
END;

CREATE FUNCTION public.revoke_user_token_families(p_user_id uuid, p_reason text)
    RETURNS TABLE
            (
                family_id uuid
            )
    LANGUAGE sql
    VOLATILE
BEGIN
    ATOMIC
    INSERT INTO public.token_family_revocation (family_id, reason)
    SELECT f.family_id, p_reason
    FROM public.token_family f
             JOIN public.token_family_window w ON w.family_id = f.family_id
    WHERE f.user_id = p_user_id
      AND w.expires_at > now()
    ON CONFLICT (family_id) DO NOTHING
    RETURNING token_family_revocation.family_id;
END;

REVOKE ALL ON FUNCTION
    public.create_token_family(uuid, timestamptz),
    public.get_user_token_families(uuid),
    public.revoke_token_family(uuid, text),
    public.revoke_user_token_families(uuid, text)
    FROM PUBLIC;

GRANT EXECUTE ON FUNCTION
    public.create_token_family(uuid, timestamptz),
    public.get_user_token_families(uuid),
    public.revoke_token_family(uuid, text),
    public.revoke_user_token_families(uuid, text)
    TO auth_app;

GRANT SELECT, INSERT ON
    public.token_family,
    public.token_family_revocation,
    public.token_family_window
    TO auth_app;
