using Cronos;

using Microsoft.Extensions.Options;

using Polly;
using Polly.Registry;
using Polly.Timeout;

namespace KeyProvider.KeyRotation;

internal sealed class RotationWorker(
    IKeyRotationService rotationService,
    ResiliencePipelineProvider<string> pipelineProvider,
    IOptions<RotationScheduleOptions> options,
    ILogger<RotationWorker> logger,
    TimeProvider timeProvider
) : BackgroundService
{
    private readonly RotationScheduleOptions _options = options.Value;

    private readonly ResiliencePipeline<KeyRotationResult> _pipeline =
        pipelineProvider.GetPipeline<KeyRotationResult>(RotationRetryOptions.PipelineName);

    private readonly CronExpression _schedule = CronExpression.Parse(options.Value.Expression);

    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
    {
        while (true)
        {
            TimeSpan delay;
            try
            {
                delay = await RunOnceAsync(stoppingToken);
            }
            catch (Exception e) when (!stoppingToken.IsCancellationRequested)
            {
                logger.LogError(e, "Unexpected error in the key rotation worker");
                delay = _options.FailureRetryDelay;
            }

            logger.LogDebug("Next check in {Delay}", delay);
            await Task.Delay(delay, timeProvider, stoppingToken);
        }
    }

    internal async Task<TimeSpan> RunOnceAsync(CancellationToken stoppingToken)
    {
        Guid tickId = Guid.CreateVersion7(timeProvider.GetUtcNow());

        KeyRotationJob? latest = await rotationService.GetLatestSuccessfulJobAsync(stoppingToken);
        DateTimeOffset now = timeProvider.GetUtcNow();

        DateTimeOffset? dueAt = latest is null
            ? now
            : _schedule.GetNextOccurrence(latest.FinishedAt, TimeZoneInfo.Utc);

        if (dueAt is null)
        {
            logger.LogWarning("[{TickId}] - Schedule '{Expression}' has no upcoming slot", tickId,
                _options.Expression);
            return _options.MaxCheckDelay;
        }

        if (dueAt > now)
        {
            logger.LogDebug("[{TickId}] - Current key pair is valid, next rotation due at {DueAt}", tickId, dueAt);
            return UntilCapped(dueAt.Value, now);
        }

        logger.LogInformation("[{TickId}] - Rotation due since {DueAt}, rotating", tickId, dueAt);
        KeyRotationResult? result = await RotateAsync(stoppingToken);
        if (result is not KeyRotationSucceeded)
        {
            return _options.FailureRetryDelay;
        }

        now = timeProvider.GetUtcNow();
        DateTimeOffset? next = _schedule.GetNextOccurrence(now, TimeZoneInfo.Utc);
        return next is null ? _options.MaxCheckDelay : UntilCapped(next.Value, now);
    }

    private TimeSpan UntilCapped(DateTimeOffset at, DateTimeOffset now)
    {
        TimeSpan delay = at - now;
        return delay > _options.MaxCheckDelay ? _options.MaxCheckDelay : delay;
    }

    private async Task<KeyRotationResult?> RotateAsync(CancellationToken stoppingToken)
    {
        try
        {
            KeyRotationResult result = await _pipeline.ExecuteAsync(
                static (service, ct) => new ValueTask<KeyRotationResult>(service.RotateAsync(ct)),
                rotationService,
                stoppingToken);

            switch (result)
            {
                case KeyRotationSucceeded success:
                    logger.LogInformation("Rotated to key {KeyId} in {Duration}", success.NewKeyId,
                        success.Duration);
                    break;
                case KeyRotationFailed failure:
                    logger.LogError("Key rotation gave up (transient: {IsTransient}): {Reason}",
                        failure.IsTransient, failure.Reason);
                    break;
            }

            return result;
        }
        catch (TimeoutRejectedException e)
        {
            logger.LogError(e, "Key rotation timed out on every attempt");
            return null;
        }
    }
}
