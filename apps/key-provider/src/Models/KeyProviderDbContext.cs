using Microsoft.EntityFrameworkCore;

using Weatherlab.EntityFrameworkCore.AppendOnly;

namespace KeyProvider.Models;

internal sealed class KeyProviderDbContext(DbContextOptions<KeyProviderDbContext> options) : DbContext(options)
{
    public DbSet<KeyRotationJob> KeyRotationJobs => Set<KeyRotationJob>();
    public DbSet<KeyPairRecord> KeyPairRecords => Set<KeyPairRecord>();

    protected override void ConfigureConventions(ModelConfigurationBuilder configurationBuilder)
    {
        configurationBuilder.Conventions.Add(_ => new AppendOnlyConvention());
    }
}
