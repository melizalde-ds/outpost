namespace KeyProvider.Worker;

public record KeyRotationOptions
{
    public TimeSpan Interval { get; init; } = TimeSpan.FromMinutes(5);
}
