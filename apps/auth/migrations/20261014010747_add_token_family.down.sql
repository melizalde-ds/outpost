DROP FUNCTION public.revoke_user_token_families(uuid, text);
DROP FUNCTION public.revoke_token_family(uuid, text);
DROP FUNCTION public.get_user_token_families(uuid);
DROP FUNCTION public.create_token_family(uuid, timestamptz);

DROP TABLE public.token_family_window;
DROP TABLE public.token_family_revocation;
DROP TABLE public.token_family;
