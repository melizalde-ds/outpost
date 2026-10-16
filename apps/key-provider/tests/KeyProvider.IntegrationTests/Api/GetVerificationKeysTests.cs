using Grpc.Core;
using KeyProvider.IntegrationTests.Infrastructure;
using Npgsql;
using Outpost.Key.V1;
using Shouldly;

namespace KeyProvider.IntegrationTests.Api;

[Trait("Consumer", "auth")]
public sealed class GetVerificationKeysTests(PostgresFixture postgres)
{
    private static readonly TimeSpan KeyLifespan = TimeSpan.FromDays(1);

    private readonly CancellationToken _ct = TestContext.Current.CancellationToken;

    private async Task<ServiceFactory> StartAsync()
    {
        return new ServiceFactory(await postgres.CreateDatabasesAsync(_ct));
    }

    private async Task<string[]> ListedKeyIdsAsync(ServiceFactory service)
    {
        GetVerificationKeysResponse response = await service
            .CreateKeyClient()
            .GetVerificationKeysAsync(new GetVerificationKeysRequest(), cancellationToken: _ct);
        return response.Keys.Select(k => k.KeyId).ToArray();
    }

    [Fact]
    public async Task GetVerificationKeys_NoKeys_ReturnsEmpty()
    {
        // Arrange
        await using ServiceFactory service = await StartAsync();

        // Act
        string[] keyIds = await ListedKeyIdsAsync(service);

        // Assert
        keyIds.ShouldBeEmpty();
    }

    [Fact]
    public async Task GetVerificationKeys_TwoLiveKeys_ReturnsBothOldestFirst()
    {
        // Arrange
        await using ServiceFactory service = await StartAsync();
        Guid older = await service.RotateAsync(_ct);
        service.Time.Advance(TimeSpan.FromHours(1));
        Guid newer = await service.RotateAsync(_ct);

        // Act
        string[] keyIds = await ListedKeyIdsAsync(service);

        // Assert
        keyIds.ShouldBe([older.ToString(), newer.ToString()]);
    }

    [Fact]
    public async Task GetVerificationKeys_OneExpiredAndOneLiveKey_ReturnsOnlyTheLiveKey()
    {
        // Arrange
        await using ServiceFactory service = await StartAsync();
        await service.RotateAsync(_ct);
        service.Time.Advance(TimeSpan.FromHours(12));
        Guid live = await service.RotateAsync(_ct);
        service.Time.SetUtcNow(ServiceFactory.Start + KeyLifespan + TimeSpan.FromSeconds(1));

        // Act
        string[] keyIds = await ListedKeyIdsAsync(service);

        // Assert
        keyIds.ShouldBe([live.ToString()]);
    }

    [Fact]
    public async Task GetVerificationKeys_KeyExpiringExactlyNow_IsNotListed()
    {
        // Arrange
        await using ServiceFactory service = await StartAsync();
        await service.RotateAsync(_ct);
        service.Time.SetUtcNow(ServiceFactory.Start + KeyLifespan);

        // Act
        string[] keyIds = await ListedKeyIdsAsync(service);

        // Assert
        keyIds.ShouldBeEmpty();
    }

    [Fact]
    public async Task GetVerificationKeys_DatabaseUnreachable_ReturnsUnavailable()
    {
        // Arrange
        TestDatabases databases = await postgres.CreateDatabasesAsync(_ct);
        string closedPort = new NpgsqlConnectionStringBuilder(databases.KeyProvider)
        {
            Host = "127.0.0.1",
            Port = 1,
        }.ConnectionString;
        await using ServiceFactory service = new(databases with { KeyProvider = closedPort });

        // Act
        RpcException error = await Should.ThrowAsync<RpcException>(() =>
            ListedKeyIdsAsync(service)
        );

        // Assert
        error.StatusCode.ShouldBe(StatusCode.Unavailable);
    }
}
