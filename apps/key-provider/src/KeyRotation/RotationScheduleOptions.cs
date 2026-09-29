using System.ComponentModel.DataAnnotations;

namespace KeyProvider.KeyRotation;

public sealed record RotationScheduleOptions
{
    [Required] public string Expression { get; init; } = "0 */8 * * *";

    [Range(typeof(TimeSpan), "00:00:01", "01:00:00")]
    public TimeSpan MaxCheckDelay { get; init; } = TimeSpan.FromMinutes(5);

    [Range(typeof(TimeSpan), "00:00:01", "01:00:00")]
    public TimeSpan FailureRetryDelay { get; init; } = TimeSpan.FromMinutes(5);
}
