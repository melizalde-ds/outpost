CREATE TABLE IF NOT EXISTS "__EFMigrationsHistory" (
    migration_id character varying(150) NOT NULL,
    product_version character varying(32) NOT NULL,
    CONSTRAINT pk___ef_migrations_history PRIMARY KEY (migration_id)
);

START TRANSACTION;

DO $EF$
BEGIN
    IF NOT EXISTS(SELECT 1 FROM "__EFMigrationsHistory" WHERE "migration_id" = '20260929000626_KeyProviderInitial') THEN
    CREATE TABLE key_rotation_job (
        id uuid NOT NULL,
        started_at timestamp with time zone NOT NULL,
        finished_at timestamp with time zone NOT NULL,
        success boolean NOT NULL,
        error_message character varying(512),
        CONSTRAINT pk_key_rotation_job PRIMARY KEY (id),
        CONSTRAINT ck_key_rotation_job_error_message CHECK ((success AND error_message IS NULL) OR (NOT success AND error_message IS NOT NULL)),
        CONSTRAINT ck_key_rotation_job_finished_after_started CHECK (finished_at >= started_at)
    );
    END IF;
END $EF$;

DO $EF$
BEGIN
    IF NOT EXISTS(SELECT 1 FROM "__EFMigrationsHistory" WHERE "migration_id" = '20260929000626_KeyProviderInitial') THEN
    CREATE TABLE key_pair_record (
        id uuid NOT NULL,
        created_at timestamp with time zone NOT NULL,
        public_key bytea NOT NULL,
        private_key bytea NOT NULL,
        expires_at timestamp with time zone NOT NULL,
        key_pair_rotation_job_id uuid NOT NULL,
        CONSTRAINT pk_key_pair_record PRIMARY KEY (id),
        CONSTRAINT ck_key_pair_record_expires_after_created CHECK (expires_at > created_at),
        CONSTRAINT ck_key_pair_record_public_key_length CHECK (octet_length(public_key) = 32),
        CONSTRAINT fk_key_pair_record_key_rotation_job_key_pair_rotation_job_id FOREIGN KEY (key_pair_rotation_job_id) REFERENCES key_rotation_job (id) ON DELETE RESTRICT
    );
    END IF;
END $EF$;

DO $EF$
BEGIN
    IF NOT EXISTS(SELECT 1 FROM "__EFMigrationsHistory" WHERE "migration_id" = '20260929000626_KeyProviderInitial') THEN
    CREATE INDEX ix_key_pair_record_created_at ON key_pair_record (created_at DESC);
    END IF;
END $EF$;

DO $EF$
BEGIN
    IF NOT EXISTS(SELECT 1 FROM "__EFMigrationsHistory" WHERE "migration_id" = '20260929000626_KeyProviderInitial') THEN
    CREATE INDEX ix_key_pair_record_expires_at ON key_pair_record (expires_at);
    END IF;
END $EF$;

DO $EF$
BEGIN
    IF NOT EXISTS(SELECT 1 FROM "__EFMigrationsHistory" WHERE "migration_id" = '20260929000626_KeyProviderInitial') THEN
    CREATE UNIQUE INDEX ix_key_pair_record_key_pair_rotation_job_id ON key_pair_record (key_pair_rotation_job_id);
    END IF;
END $EF$;

DO $EF$
BEGIN
    IF NOT EXISTS(SELECT 1 FROM "__EFMigrationsHistory" WHERE "migration_id" = '20260929000626_KeyProviderInitial') THEN
    CREATE INDEX ix_key_rotation_job_finished_at_successful ON key_rotation_job (finished_at DESC) WHERE success;
    END IF;
END $EF$;

DO $EF$
BEGIN
    IF NOT EXISTS(SELECT 1 FROM "__EFMigrationsHistory" WHERE "migration_id" = '20260929000626_KeyProviderInitial') THEN
    CREATE FUNCTION public.append_only_guard() RETURNS trigger
    LANGUAGE plpgsql AS $fn$
    BEGIN
        RAISE EXCEPTION USING
            ERRCODE = 'OPA01',
            MESSAGE = format('%s on append-only table %I.%I is not allowed', TG_OP, TG_TABLE_SCHEMA, TG_TABLE_NAME),
            SCHEMA = TG_TABLE_SCHEMA,
            TABLE = TG_TABLE_NAME;
    END;
    $fn$;
    END IF;
END $EF$;

DO $EF$
BEGIN
    IF NOT EXISTS(SELECT 1 FROM "__EFMigrationsHistory" WHERE "migration_id" = '20260929000626_KeyProviderInitial') THEN
    CREATE FUNCTION public.key_pair_record_require_successful_job() RETURNS trigger
    LANGUAGE plpgsql AS $fn$
    BEGIN
        IF NOT EXISTS (
            SELECT 1 FROM public.key_rotation_job
            WHERE id = NEW.key_pair_rotation_job_id AND success
        ) THEN
            RAISE EXCEPTION USING
                ERRCODE = 'OPA02',
                MESSAGE = format('key pair %s references rotation job %s, which is missing or did not succeed', NEW.id, NEW.key_pair_rotation_job_id),
                SCHEMA = TG_TABLE_SCHEMA,
                TABLE = TG_TABLE_NAME,
                COLUMN = 'key_pair_rotation_job_id';
        END IF;
        RETURN NEW;
    END;
    $fn$;
    END IF;
END $EF$;

DO $EF$
BEGIN
    IF NOT EXISTS(SELECT 1 FROM "__EFMigrationsHistory" WHERE "migration_id" = '20260929000626_KeyProviderInitial') THEN
    CREATE TRIGGER key_rotation_job_append_only
    BEFORE UPDATE OR DELETE ON public.key_rotation_job
    FOR EACH ROW EXECUTE FUNCTION public.append_only_guard();
    END IF;
END $EF$;

DO $EF$
BEGIN
    IF NOT EXISTS(SELECT 1 FROM "__EFMigrationsHistory" WHERE "migration_id" = '20260929000626_KeyProviderInitial') THEN
    CREATE TRIGGER key_rotation_job_no_truncate
    BEFORE TRUNCATE ON public.key_rotation_job
    FOR EACH STATEMENT EXECUTE FUNCTION public.append_only_guard();
    END IF;
END $EF$;

DO $EF$
BEGIN
    IF NOT EXISTS(SELECT 1 FROM "__EFMigrationsHistory" WHERE "migration_id" = '20260929000626_KeyProviderInitial') THEN
    CREATE TRIGGER key_pair_record_append_only
    BEFORE UPDATE OR DELETE ON public.key_pair_record
    FOR EACH ROW EXECUTE FUNCTION public.append_only_guard();
    END IF;
END $EF$;

DO $EF$
BEGIN
    IF NOT EXISTS(SELECT 1 FROM "__EFMigrationsHistory" WHERE "migration_id" = '20260929000626_KeyProviderInitial') THEN
    CREATE TRIGGER key_pair_record_no_truncate
    BEFORE TRUNCATE ON public.key_pair_record
    FOR EACH STATEMENT EXECUTE FUNCTION public.append_only_guard();
    END IF;
END $EF$;

DO $EF$
BEGIN
    IF NOT EXISTS(SELECT 1 FROM "__EFMigrationsHistory" WHERE "migration_id" = '20260929000626_KeyProviderInitial') THEN
    CREATE TRIGGER key_pair_record_require_successful_job
    BEFORE INSERT ON public.key_pair_record
    FOR EACH ROW EXECUTE FUNCTION public.key_pair_record_require_successful_job();
    END IF;
END $EF$;

DO $EF$
BEGIN
    IF NOT EXISTS(SELECT 1 FROM "__EFMigrationsHistory" WHERE "migration_id" = '20260929000626_KeyProviderInitial') THEN
    GRANT SELECT, INSERT ON public.key_rotation_job, public.key_pair_record TO key_provider_app;
    END IF;
END $EF$;

DO $EF$
BEGIN
    IF NOT EXISTS(SELECT 1 FROM "__EFMigrationsHistory" WHERE "migration_id" = '20260929000626_KeyProviderInitial') THEN
    INSERT INTO "__EFMigrationsHistory" (migration_id, product_version)
    VALUES ('20260929000626_KeyProviderInitial', '10.0.12');
    END IF;
END $EF$;
COMMIT;

