using KeyProvider.Models;

using Microsoft.EntityFrameworkCore;

using Weatherlab.EntityFrameworkCore.AppendOnly;
using Weatherlab.EntityFrameworkCore.CreatedAt;

namespace KeyProvider.Data;

internal sealed class KeyProviderDbContext(DbContextOptions<KeyProviderDbContext> options) : DbContext(options)
{
    public DbSet<KeyRotationJob> KeyRotationJobs => Set<KeyRotationJob>();
    public DbSet<KeyPairRecord> KeyPairRecords => Set<KeyPairRecord>();

    protected override void ConfigureConventions(ModelConfigurationBuilder configurationBuilder)
    {
        configurationBuilder.Conventions.Add(_ => new AppendOnlyConvention());
        configurationBuilder.Conventions.Add(_ => new CreatedAtConvention());
    }
}
