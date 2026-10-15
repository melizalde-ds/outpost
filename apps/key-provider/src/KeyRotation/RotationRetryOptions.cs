using System.ComponentModel.DataAnnotations;

using Polly;
using Polly.Retry;
using Polly.Timeout;

namespace KeyProvider.KeyRotation;

public sealed record RotationRetryOptions
{
    public const string PipelineName = "key-rotation";

    [Range(1, 10)] public int MaxRetryAttempts { get; init; } = 3;

    [Range(typeof(TimeSpan), "00:00:00.100", "00:10:00")]
    public TimeSpan BaseDelay { get; init; } = TimeSpan.FromSeconds(2);

    [Range(typeof(TimeSpan), "00:00:01", "01:00:00")]
    public TimeSpan MaxDelay { get; init; } = TimeSpan.FromMinutes(1);

    [Range(typeof(TimeSpan), "00:00:01", "00:10:00")]
    public TimeSpan AttemptTimeout { get; init; } = TimeSpan.FromSeconds(30);

    internal void ConfigurePipeline(ResiliencePipelineBuilder<KeyRotationResult> builder)
    {
        builder
            .AddRetry(new RetryStrategyOptions<KeyRotationResult>
            {
                ShouldHandle = new PredicateBuilder<KeyRotationResult>()
                    .HandleResult(r => r is KeyRotationFailed { IsTransient: true })
                    .Handle<TimeoutRejectedException>(),
                MaxRetryAttempts = MaxRetryAttempts,
                BackoffType = DelayBackoffType.Exponential,
                UseJitter = true,
                Delay = BaseDelay,
                MaxDelay = MaxDelay
            })
            .AddTimeout(AttemptTimeout);
    }
}
