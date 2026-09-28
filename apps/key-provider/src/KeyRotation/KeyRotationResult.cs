namespace KeyProvider.KeyRotation;

public abstract record KeyRotationResult(DateTimeOffset StartedAt, DateTimeOffset CompletedAt)
{
    public TimeSpan Duration => CompletedAt - StartedAt;
}

public sealed record KeyRotationSucceeded(
    DateTimeOffset StartedAt,
    DateTimeOffset CompletedAt,
    Guid NewKeyId)
    : KeyRotationResult(StartedAt, CompletedAt);

public sealed record KeyRotationFailed(
    DateTimeOffset StartedAt,
    DateTimeOffset CompletedAt,
    string Reason,
    bool IsTransient,
    Exception? Exception = null)
    : KeyRotationResult(StartedAt, CompletedAt);
