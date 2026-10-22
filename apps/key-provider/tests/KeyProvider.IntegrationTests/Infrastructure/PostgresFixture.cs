using KeyProvider.Database;
using KeyProvider.IntegrationTests.Infrastructure;

using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;

using Npgsql;

using Testcontainers.PostgreSql;

[assembly: AssemblyFixture(typeof(PostgresFixture))]

namespace KeyProvider.IntegrationTests.Infrastructure;

public sealed record TestDatabases(
    string KeyProvider,
    string DataProtection,
    string KeyProviderAdmin
);

public sealed class PostgresFixture : IAsyncLifetime
{
    private const string KeyProviderTemplate = "key_provider";
    private const string DataProtectionTemplate = "data_protection";
    private const string Password = "test-password";

    private readonly PostgreSqlContainer _postgres = new PostgreSqlBuilder("postgres:18")
        .WithDatabase(KeyProviderTemplate)
        .WithEnvironment("MIGRATION_PASSWORD", Password)
        .WithEnvironment("APP_PASSWORD", Password)
        .WithEnvironment("POSTGRES_ENCRYPTION_DB", DataProtectionTemplate)
        .WithEnvironment("DP_MIGRATION_PASSWORD", Password)
        .WithEnvironment("DP_APP_PASSWORD", Password)
        .WithResourceMapping(
            new DirectoryInfo(Path.Combine(AppContext.BaseDirectory, "db", "init")),
            "/docker-entrypoint-initdb.d/"
        )
        .Build();

    public async ValueTask InitializeAsync()
    {
        await _postgres.StartAsync();
        await MigrateAsync();
        NpgsqlConnection.ClearAllPools();
    }

    public ValueTask DisposeAsync()
    {
        return _postgres.DisposeAsync();
    }

    public async Task<TestDatabases> CreateDatabasesAsync(CancellationToken ct)
    {
        string suffix = Guid.CreateVersion7().ToString("N");
        string keyProvider = $"kp_{suffix}";
        string dataProtection = $"dp_{suffix}";
        await using NpgsqlConnection admin = new(
            ConnectionString("postgres", "postgres", "postgres")
        );
        await admin.OpenAsync(ct);
        await CopyAsync(KeyProviderTemplate, keyProvider);
        await CopyAsync(DataProtectionTemplate, dataProtection);

        return new TestDatabases(
            ConnectionString(keyProvider, "key_provider_app", Password),
            ConnectionString(dataProtection, "dp_app", Password),
            ConnectionString(keyProvider, "postgres", "postgres")
        );

        async Task CopyAsync(string template, string name)
        {
#pragma warning disable CA2100
            await using NpgsqlCommand create = new(
                $"""CREATE DATABASE "{name}" TEMPLATE "{template}" """,
                admin
            );
#pragma warning restore CA2100
            await create.ExecuteNonQueryAsync(ct);
        }
    }

    private async Task MigrateAsync()
    {
        IConfiguration configuration = new ConfigurationBuilder()
            .AddInMemoryCollection(
                new Dictionary<string, string?>
                {
                    ["ConnectionStrings:KeyProvider"] = ConnectionString(
                        KeyProviderTemplate,
                        "key_provider_migrator",
                        Password
                    ),
                    ["ConnectionStrings:DataProtection"] = ConnectionString(
                        DataProtectionTemplate,
                        "dp_migrator",
                        Password
                    ),
                }
            )
            .Build();
        ServiceCollection services = new();
        services.AddSingleton(TimeProvider.System);
        services.AddKeyProviderDbContext(configuration);
        services.AddDataProtectionDbContext(configuration);
        await using ServiceProvider provider = services.BuildServiceProvider();
        await using AsyncServiceScope scope = provider.CreateAsyncScope();
        await scope
            .ServiceProvider.GetRequiredService<KeyProviderDbContext>()
            .Database.MigrateAsync();
        await scope
            .ServiceProvider.GetRequiredService<DataProtectionDbContext>()
            .Database.MigrateAsync();
    }

    private string ConnectionString(string database, string username, string password)
    {
        return new NpgsqlConnectionStringBuilder(_postgres.GetConnectionString())
        {
            Database = database,
            Username = username,
            Password = password,
        }.ConnectionString;
    }
}
