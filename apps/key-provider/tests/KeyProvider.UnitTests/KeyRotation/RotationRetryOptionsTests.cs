using KeyProvider.KeyRotation;

using Polly;
using Polly.Retry;
using Polly.Testing;
using Polly.Timeout;

using Shouldly;

namespace KeyProvider.UnitTests.KeyRotation;

public sealed class RotationRetryOptionsTests
{
    private static readonly DateTimeOffset Now = new(2026, 1, 1, 0, 0, 0, TimeSpan.Zero);

    private static readonly RotationRetryOptions Retry = new()
    {
        MaxRetryAttempts = 4,
        BaseDelay = TimeSpan.FromSeconds(3),
        MaxDelay = TimeSpan.FromSeconds(40),
        AttemptTimeout = TimeSpan.FromSeconds(20)
    };

    private static ResiliencePipeline<KeyRotationResult> BuildPipeline()
    {
        ResiliencePipelineBuilder<KeyRotationResult> builder = new();
        Retry.ConfigurePipeline(builder);
        return builder.Build();
    }

    private static ValueTask<bool> ShouldRetryAsync(Outcome<KeyRotationResult> outcome)
    {
        RetryStrategyOptions<KeyRotationResult> options = BuildPipeline().GetPipelineDescriptor()
            .Strategies[0].Options.ShouldBeOfType<RetryStrategyOptions<KeyRotationResult>>();
        return options.ShouldHandle(new RetryPredicateArguments<KeyRotationResult>(
            ResilienceContextPool.Shared.Get(TestContext.Current.CancellationToken), outcome, attemptNumber: 0));
    }

    [Fact]
    public void ConfigurePipeline_FromOptions_RetriesWithExponentialJitterAroundATimeoutPerAttempt()
    {
        // Arrange
        ResiliencePipeline<KeyRotationResult> pipeline = BuildPipeline();

        // Act
        ResiliencePipelineDescriptor descriptor = pipeline.GetPipelineDescriptor();

        // Assert
        descriptor.Strategies.Count.ShouldBe(2);
        RetryStrategyOptions<KeyRotationResult> retry =
            descriptor.Strategies[0].Options.ShouldBeOfType<RetryStrategyOptions<KeyRotationResult>>();
        retry.MaxRetryAttempts.ShouldBe(Retry.MaxRetryAttempts);
        retry.BackoffType.ShouldBe(DelayBackoffType.Exponential);
        retry.UseJitter.ShouldBeTrue();
        retry.Delay.ShouldBe(Retry.BaseDelay);
        retry.MaxDelay.ShouldBe(Retry.MaxDelay);
        descriptor.Strategies[1].Options.ShouldBeOfType<TimeoutStrategyOptions>().Timeout
            .ShouldBe(Retry.AttemptTimeout);
    }

    [Theory]
    [InlineData(true, true)]
    [InlineData(false, false)]
    public async Task ConfigurePipeline_RotationFailed_RetriesOnlyTransientFailures(bool isTransient, bool retried)
    {
        // Arrange
        Outcome<KeyRotationResult> failed =
            Outcome.FromResult<KeyRotationResult>(new KeyRotationFailed(Now, Now, "failed", isTransient));

        // Act
        bool result = await ShouldRetryAsync(failed);

        // Assert
        result.ShouldBe(retried);
    }

    [Fact]
    public async Task ConfigurePipeline_RotationSucceeded_DoesNotRetry()
    {
        // Arrange
        Outcome<KeyRotationResult> succeeded =
            Outcome.FromResult<KeyRotationResult>(new KeyRotationSucceeded(Now, Now, Guid.Empty));

        // Act
        bool result = await ShouldRetryAsync(succeeded);

        // Assert
        result.ShouldBeFalse();
    }

    [Fact]
    public async Task ConfigurePipeline_AttemptTimedOut_Retries()
    {
        // Arrange
        Outcome<KeyRotationResult> timedOut = Outcome.FromException<KeyRotationResult>(new TimeoutRejectedException());

        // Act
        bool result = await ShouldRetryAsync(timedOut);

        // Assert
        result.ShouldBeTrue();
    }
}
