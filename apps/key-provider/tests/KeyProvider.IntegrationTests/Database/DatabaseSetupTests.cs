using KeyProvider.Database;
using KeyProvider.IntegrationTests.Infrastructure;
using Microsoft.AspNetCore.DataProtection;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Options;
using Shouldly;

namespace KeyProvider.IntegrationTests.Database;

public sealed class DatabaseSetupTests(PostgresFixture postgres)
{
    private readonly CancellationToken _ct = TestContext.Current.CancellationToken;

    [Fact]
    public async Task KeyProviderModel_ComparedWithTheMigrations_HasNoChangesWithoutAMigration()
    {
        // Arrange
        await using ServiceFactory service = new(await postgres.CreateDatabasesAsync(_ct));
        await using KeyProviderDbContext context = await service
            .Services.GetRequiredService<IDbContextFactory<KeyProviderDbContext>>()
            .CreateDbContextAsync(_ct);

        // Act
        bool pending = context.Database.HasPendingModelChanges();

        // Assert
        pending.ShouldBeFalse();
    }

    [Fact]
    public async Task DataProtection_ApplicationName_IsKeyProvider()
    {
        // Arrange
        await using ServiceFactory service = new(await postgres.CreateDatabasesAsync(_ct));

        // Act
        string? name = service
            .Services.GetRequiredService<IOptions<DataProtectionOptions>>()
            .Value.ApplicationDiscriminator;

        // Assert
        name.ShouldBe("key-provider");
    }
}
