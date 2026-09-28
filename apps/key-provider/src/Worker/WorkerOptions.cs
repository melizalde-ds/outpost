using System.ComponentModel.DataAnnotations;

namespace KeyProvider.Worker;

public sealed record WorkerOptions
{
    // Standard 5-field cron expression, evaluated in UTC. Parsed and validated by WorkerOptionsValidator.
    [Required] public required string Expression { get; init; } = "0 */8 * * *";

    [Range(typeof(TimeSpan), "00:00:01", "01:00:00")]
    [Required]
    public required TimeSpan MaxCheckDelay { get; init; } = TimeSpan.FromMinutes(5);

    [Range(typeof(TimeSpan), "00:00:01", "01:00:00")]
    [Required]
    public required TimeSpan FailureRetryDelay { get; init; } = TimeSpan.FromMinutes(5);
}
