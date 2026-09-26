namespace KeyProvider.Worker;

public sealed record KeyRotationOptions
{
    public TimeSpan Interval { get; init; } = TimeSpan.FromMinutes(5);
}
