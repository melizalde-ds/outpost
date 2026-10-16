using KeyProvider.IntegrationTests.Infrastructure;
using KeyProvider.KeyRotation;
using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.Mvc.Testing;
using Microsoft.Extensions.Options;
using Outpost.Key.V1;
using Shouldly;

namespace KeyProvider.IntegrationTests;

public sealed class StartupTests(PostgresFixture postgres)
{
    private static readonly TimeSpan FirstKeyLimit = TimeSpan.FromSeconds(10);

    private readonly CancellationToken _ct = TestContext.Current.CancellationToken;

    private async Task<int> WaitForKeysAsync(ServiceFactory service)
    {
        using CancellationTokenSource limit = CancellationTokenSource.CreateLinkedTokenSource(_ct);
        limit.CancelAfter(FirstKeyLimit);
        KeyService.KeyServiceClient client = service.CreateKeyClient();
        while (true)
        {
            GetVerificationKeysResponse response = await client.GetVerificationKeysAsync(
                new GetVerificationKeysRequest(),
                cancellationToken: limit.Token
            );
            if (response.Keys.Count > 0)
            {
                return response.Keys.Count;
            }

            await Task.Delay(TimeSpan.FromMilliseconds(100), limit.Token);
        }
    }

    [Fact]
    public async Task Start_NoKeysYet_RotationWorkerMakesTheFirstKey()
    {
        // Arrange
        await using ServiceFactory service = new(
            await postgres.CreateDatabasesAsync(_ct),
            keepRotationWorker: true
        );

        // Act
        int keyCount = await WaitForKeysAsync(service);

        // Assert
        keyCount.ShouldBe(1);
    }

    [Fact]
    public async Task Start_KeyLifespanShorterThanTheSchedule_RefusesToStart()
    {
        // Arrange
        await using ServiceFactory service = new(await postgres.CreateDatabasesAsync(_ct));
        await using WebApplicationFactory<Program> misconfigured = service.WithWebHostBuilder(b =>
            b.UseSetting("KeyRotation:Lifespan", "01:00:00")
        );

        // Act
        OptionsValidationException error = Should.Throw<OptionsValidationException>(() =>
            misconfigured.Services
        );

        // Assert
        error.OptionsType.ShouldBe(typeof(RotationScheduleOptions));
    }
}
