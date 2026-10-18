using KeyProvider.IntegrationTests.Infrastructure;
using KeyProvider.KeyRotation;
using Microsoft.Extensions.DependencyInjection;
using Npgsql;
using Shouldly;

namespace KeyProvider.IntegrationTests.KeyRotation;

public sealed class RotationJobStoreTests(PostgresFixture postgres)
{
    private readonly CancellationToken _ct = TestContext.Current.CancellationToken;

    private static KeyRotationJob Job(bool success)
    {
        return new KeyRotationJob
        {
            KeyPairRotationJobId = Guid.CreateVersion7(),
            StartedAt = ServiceFactory.Start,
            Success = success,
            ErrorMessage = success ? null : "TimeoutException: too slow",
        };
    }

    [Fact]
    public async Task SaveFailedRotationAsync_FailedJob_IsStoredWithItsReason()
    {
        // Arrange
        TestDatabases databases = await postgres.CreateDatabasesAsync(_ct);
        await using ServiceFactory service = new(databases);
        await using AsyncServiceScope scope = service.Services.CreateAsyncScope();
        IRotationJobStore store = scope.ServiceProvider.GetRequiredService<IRotationJobStore>();

        // Act
        await store.SaveFailedRotationAsync(Job(success: false), _ct);

        // Assert
        await using NpgsqlConnection admin = new(databases.KeyProviderAdmin);
        await admin.OpenAsync(_ct);
        await using NpgsqlCommand select = new(
            "SELECT success, error_message FROM key_rotation_job",
            admin
        );
        await using NpgsqlDataReader row = await select.ExecuteReaderAsync(_ct);
        (await row.ReadAsync(_ct)).ShouldBeTrue();
        ((bool)row["success"]).ShouldBeFalse();
        ((string)row["error_message"]).ShouldBe("TimeoutException: too slow");
    }

    [Fact]
    public async Task GetLatestSuccessfulJobAsync_RotatedThenFailed_ReturnsTheSuccessfulJob()
    {
        // Arrange
        await using ServiceFactory service = new(await postgres.CreateDatabasesAsync(_ct));
        await service.RotateAsync(_ct);
        await using AsyncServiceScope scope = service.Services.CreateAsyncScope();
        IRotationJobStore store = scope.ServiceProvider.GetRequiredService<IRotationJobStore>();
        await store.SaveFailedRotationAsync(Job(success: false), _ct);

        // Act
        KeyRotationJob? latest = await store.GetLatestSuccessfulJobAsync(_ct);

        // Assert
        latest.ShouldNotBeNull().Success.ShouldBeTrue();
    }

    [Fact]
    public async Task SaveFailedRotationAsync_SuccessfulJob_Throws()
    {
        // Arrange
        await using ServiceFactory service = new(await postgres.CreateDatabasesAsync(_ct));
        await using AsyncServiceScope scope = service.Services.CreateAsyncScope();
        IRotationJobStore store = scope.ServiceProvider.GetRequiredService<IRotationJobStore>();

        // Act
        ArgumentException error = await Should.ThrowAsync<ArgumentException>(() =>
            store.SaveFailedRotationAsync(Job(success: true), _ct)
        );

        // Assert
        error.ParamName.ShouldBe("job");
        error.Message.ShouldStartWith("Expected a failed rotation job.");
    }
}
