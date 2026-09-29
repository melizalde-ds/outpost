using KeyProvider.KeyRotation;
using KeyProvider.SigningKeys;

using Microsoft.EntityFrameworkCore;

using Weatherlab.EntityFrameworkCore.AppendOnly;
using Weatherlab.EntityFrameworkCore.CreatedAt;

namespace KeyProvider.Database;

internal sealed class KeyProviderDbContext(DbContextOptions<KeyProviderDbContext> options) : DbContext(options)
{
    public DbSet<KeyRotationJob> KeyRotationJobs => Set<KeyRotationJob>();
    public DbSet<KeyPairRecord> KeyPairRecords => Set<KeyPairRecord>();

    protected override void ConfigureConventions(ModelConfigurationBuilder configurationBuilder)
    {
        configurationBuilder.Conventions.Add(_ => new AppendOnlyConvention());
        configurationBuilder.Conventions.Add(_ => new CreatedAtConvention());
    }

    protected override void OnModelCreating(ModelBuilder modelBuilder)
    {
        modelBuilder.Entity<KeyRotationJob>(job =>
        {
            job.ToTable(t =>
            {
                t.HasCheckConstraint("ck_key_rotation_job_finished_after_started", "finished_at >= started_at");
                t.HasCheckConstraint("ck_key_rotation_job_error_message",
                    "(success AND error_message IS NULL) OR (NOT success AND error_message IS NOT NULL)");
            });

            job.HasIndex(j => j.FinishedAt)
                .IsDescending()
                .HasFilter("success")
                .HasDatabaseName("ix_key_rotation_job_finished_at_successful");
        });

        modelBuilder.Entity<KeyPairRecord>(record => record.ToTable(t =>
        {
            t.HasCheckConstraint("ck_key_pair_record_expires_after_created", "expires_at > created_at");
            t.HasCheckConstraint("ck_key_pair_record_public_key_length", "octet_length(public_key) = 32");
        }));
    }
}
