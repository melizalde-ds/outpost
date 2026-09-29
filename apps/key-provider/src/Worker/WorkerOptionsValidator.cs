using Cronos;

using KeyProvider.KeyRotation;
using KeyProvider.Signing;

using Microsoft.Extensions.Options;

namespace KeyProvider.Worker;

internal sealed class WorkerOptionsValidator(
    IOptions<KeyRotationOptions> rotationOptions,
    IOptions<RotationRetryOptions> retryOptions,
    IOptions<SigningServiceOptions> signingOptions,
    TimeProvider timeProvider)
    : IValidateOptions<WorkerOptions>
{
    private const int SampledOccurrences = 50;

    public ValidateOptionsResult Validate(string? name, WorkerOptions options)
    {
        if (string.IsNullOrWhiteSpace(options.Expression))
        {
            return ValidateOptionsResult.Fail("Worker:Expression is required.");
        }

        if (!CronExpression.TryParse(options.Expression, out CronExpression? expression))
        {
            return ValidateOptionsResult.Fail($"'{options.Expression}' is not a valid cron expression.");
        }

        TimeSpan maxGap = GetMaxGap(expression);
        if (maxGap == TimeSpan.Zero)
        {
            return ValidateOptionsResult.Fail($"'{options.Expression}' must fire repeatedly.");
        }

        RotationRetryOptions retry = retryOptions.Value;
        TimeSpan worstCaseRetry = retry.MaxRetryAttempts * (retry.AttemptTimeout + retry.MaxDelay);
        TimeSpan tokenLifetime = signingOptions.Value.TokenLifetime;

        TimeSpan lifespan = rotationOptions.Value.Lifespan;
        if (lifespan <= maxGap + options.FailureRetryDelay + worstCaseRetry + tokenLifetime)
        {
            return ValidateOptionsResult.Fail(
                $"KeyRotation:Lifespan ({lifespan}) must be longer than the largest gap between schedule slots " +
                $"({maxGap}) plus FailureRetryDelay ({options.FailureRetryDelay}) plus the worst-case retry " +
                $"duration ({worstCaseRetry}) plus Signing:TokenLifetime ({tokenLifetime}).");
        }

        return ValidateOptionsResult.Success;
    }

    private TimeSpan GetMaxGap(CronExpression expression)
    {
        TimeSpan maxGap = TimeSpan.Zero;
        DateTimeOffset? previous = expression.GetNextOccurrence(timeProvider.GetUtcNow(), TimeZoneInfo.Utc);

        for (int i = 0; i < SampledOccurrences && previous is not null; i++)
        {
            DateTimeOffset? next = expression.GetNextOccurrence(previous.Value, TimeZoneInfo.Utc);
            if (next is null)
            {
                break;
            }

            TimeSpan gap = next.Value - previous.Value;
            if (gap > maxGap)
            {
                maxGap = gap;
            }

            previous = next;
        }

        return maxGap;
    }
}
