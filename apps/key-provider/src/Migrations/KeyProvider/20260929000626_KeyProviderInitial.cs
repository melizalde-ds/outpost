using System;
using Microsoft.EntityFrameworkCore.Migrations;

#nullable disable

namespace KeyProvider.Migrations.KeyProvider
{
    /// <inheritdoc />
    public partial class KeyProviderInitial : Migration
    {
        /// <inheritdoc />
        protected override void Up(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.CreateTable(
                name: "key_rotation_job",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    started_at = table.Column<DateTimeOffset>(type: "timestamp with time zone", nullable: false),
                    finished_at = table.Column<DateTimeOffset>(type: "timestamp with time zone", nullable: false),
                    success = table.Column<bool>(type: "boolean", nullable: false),
                    error_message = table.Column<string>(type: "character varying(512)", maxLength: 512, nullable: true)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_key_rotation_job", x => x.id);
                    table.CheckConstraint("ck_key_rotation_job_error_message", "(success AND error_message IS NULL) OR (NOT success AND error_message IS NOT NULL)");
                    table.CheckConstraint("ck_key_rotation_job_finished_after_started", "finished_at >= started_at");
                });

            migrationBuilder.CreateTable(
                name: "key_pair_record",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    created_at = table.Column<DateTimeOffset>(type: "timestamp with time zone", nullable: false),
                    public_key = table.Column<byte[]>(type: "bytea", nullable: false),
                    private_key = table.Column<byte[]>(type: "bytea", nullable: false),
                    expires_at = table.Column<DateTimeOffset>(type: "timestamp with time zone", nullable: false),
                    key_pair_rotation_job_id = table.Column<Guid>(type: "uuid", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_key_pair_record", x => x.id);
                    table.CheckConstraint("ck_key_pair_record_expires_after_created", "expires_at > created_at");
                    table.CheckConstraint("ck_key_pair_record_public_key_length", "octet_length(public_key) = 32");
                    table.ForeignKey(
                        name: "fk_key_pair_record_key_rotation_job_key_pair_rotation_job_id",
                        column: x => x.key_pair_rotation_job_id,
                        principalTable: "key_rotation_job",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                });

            migrationBuilder.CreateIndex(
                name: "ix_key_pair_record_created_at",
                table: "key_pair_record",
                column: "created_at",
                descending: new bool[0]);

            migrationBuilder.CreateIndex(
                name: "ix_key_pair_record_expires_at",
                table: "key_pair_record",
                column: "expires_at");

            migrationBuilder.CreateIndex(
                name: "ix_key_pair_record_key_pair_rotation_job_id",
                table: "key_pair_record",
                column: "key_pair_rotation_job_id",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "ix_key_rotation_job_finished_at_successful",
                table: "key_rotation_job",
                column: "finished_at",
                descending: new bool[0],
                filter: "success");

            migrationBuilder.Sql("""
                CREATE FUNCTION public.append_only_guard() RETURNS trigger
                LANGUAGE plpgsql AS $fn$
                BEGIN
                    RAISE EXCEPTION USING
                        ERRCODE = 'WLA01',
                        MESSAGE = format('%s on append-only table %I.%I is not allowed', TG_OP, TG_TABLE_SCHEMA, TG_TABLE_NAME),
                        SCHEMA = TG_TABLE_SCHEMA,
                        TABLE = TG_TABLE_NAME;
                END;
                $fn$;
                """);

            migrationBuilder.Sql("""
                CREATE FUNCTION public.key_pair_record_require_successful_job() RETURNS trigger
                LANGUAGE plpgsql AS $fn$
                BEGIN
                    IF NOT EXISTS (
                        SELECT 1 FROM public.key_rotation_job
                        WHERE id = NEW.key_pair_rotation_job_id AND success
                    ) THEN
                        RAISE EXCEPTION USING
                            ERRCODE = 'WLA02',
                            MESSAGE = format('key pair %s references rotation job %s, which is missing or did not succeed', NEW.id, NEW.key_pair_rotation_job_id),
                            SCHEMA = TG_TABLE_SCHEMA,
                            TABLE = TG_TABLE_NAME,
                            COLUMN = 'key_pair_rotation_job_id';
                    END IF;
                    RETURN NEW;
                END;
                $fn$;
                """);

            migrationBuilder.Sql("""
                CREATE TRIGGER key_rotation_job_append_only
                BEFORE UPDATE OR DELETE ON public.key_rotation_job
                FOR EACH ROW EXECUTE FUNCTION public.append_only_guard();
                """);

            migrationBuilder.Sql("""
                CREATE TRIGGER key_rotation_job_no_truncate
                BEFORE TRUNCATE ON public.key_rotation_job
                FOR EACH STATEMENT EXECUTE FUNCTION public.append_only_guard();
                """);

            migrationBuilder.Sql("""
                CREATE TRIGGER key_pair_record_append_only
                BEFORE UPDATE OR DELETE ON public.key_pair_record
                FOR EACH ROW EXECUTE FUNCTION public.append_only_guard();
                """);

            migrationBuilder.Sql("""
                CREATE TRIGGER key_pair_record_no_truncate
                BEFORE TRUNCATE ON public.key_pair_record
                FOR EACH STATEMENT EXECUTE FUNCTION public.append_only_guard();
                """);

            migrationBuilder.Sql("""
                CREATE TRIGGER key_pair_record_require_successful_job
                BEFORE INSERT ON public.key_pair_record
                FOR EACH ROW EXECUTE FUNCTION public.key_pair_record_require_successful_job();
                """);

            migrationBuilder.Sql("""
                GRANT SELECT, INSERT ON public.key_rotation_job, public.key_pair_record TO key_provider_app;
                """);
        }

        /// <inheritdoc />
        protected override void Down(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.DropTable(
                name: "key_pair_record");

            migrationBuilder.DropTable(
                name: "key_rotation_job");

            migrationBuilder.Sql("DROP FUNCTION public.key_pair_record_require_successful_job();");

            migrationBuilder.Sql("DROP FUNCTION public.append_only_guard();");
        }
    }
}
