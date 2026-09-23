namespace Key.Worker;

public record WorkerOptions
{
    public TimeSpan Interval { get; init; } = TimeSpan.FromMinutes(5);
}
