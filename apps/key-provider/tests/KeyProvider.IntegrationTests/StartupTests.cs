using KeyProvider.IntegrationTests.Infrastructure;
using KeyProvider.KeyRotation;
using KeyProvider.Signing;
using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.Mvc.Testing;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Options;
using Outpost.Key.V1;
using Polly.Registry;
using Polly.Retry;
using Polly.Testing;
using Polly.Timeout;
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
            keepRotationWorker: true,
            fakeClock: false
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

    [Fact]
    public async Task Start_InvalidRotationSchedule_RefusesToStart()
    {
        // Arrange
        await using ServiceFactory service = new(await postgres.CreateDatabasesAsync(_ct));
        await using WebApplicationFactory<Program> misconfigured = service.WithWebHostBuilder(b =>
            b.UseSetting("KeyRotation:Schedule:Expression", "not a schedule")
        );

        // Act
        OptionsValidationException error = Should.Throw<OptionsValidationException>(() =>
            misconfigured.Services
        );

        // Assert
        error.Failures.ShouldContain("'not a schedule' is not a valid cron expression.");
    }

    [Fact]
    public async Task Start_SigningSettingsGiven_UsesThem()
    {
        // Arrange
        await using ServiceFactory service = new(await postgres.CreateDatabasesAsync(_ct));
        await using WebApplicationFactory<Program> configured = service.WithWebHostBuilder(b =>
        {
            b.UseSetting("Signing:Issuer", "test-issuer");
            b.UseSetting("Signing:TokenLifetime", "00:02:00");
        });

        // Act
        SigningOptions signing = configured
            .Services.GetRequiredService<IOptions<SigningOptions>>()
            .Value;

        // Assert
        signing.Issuer.ShouldBe("test-issuer");
        signing.TokenLifetime.ShouldBe(TimeSpan.FromMinutes(2));
    }

    [Fact]
    public async Task Start_RetrySettingsGiven_BuildsTheRotationPipelineFromThem()
    {
        // Arrange
        await using ServiceFactory service = new(await postgres.CreateDatabasesAsync(_ct));
        await using WebApplicationFactory<Program> configured = service.WithWebHostBuilder(b =>
        {
            b.UseSetting("KeyRotation:Retry:MaxRetryAttempts", "5");
            b.UseSetting("KeyRotation:Retry:AttemptTimeout", "00:00:07");
        });

        // Act
        ResiliencePipelineDescriptor descriptor = configured
            .Services.GetRequiredService<ResiliencePipelineProvider<string>>()
            .GetPipeline<KeyRotationResult>(RotationRetryOptions.PipelineName)
            .GetPipelineDescriptor();

        // Assert
        descriptor.Strategies.Count.ShouldBe(2);
        descriptor
            .Strategies[0]
            .Options.ShouldBeOfType<RetryStrategyOptions<KeyRotationResult>>()
            .MaxRetryAttempts.ShouldBe(5);
        descriptor
            .Strategies[1]
            .Options.ShouldBeOfType<TimeoutStrategyOptions>()
            .Timeout.ShouldBe(TimeSpan.FromSeconds(7));
    }
}
