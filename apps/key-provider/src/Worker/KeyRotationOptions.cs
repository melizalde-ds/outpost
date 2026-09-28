using System.ComponentModel.DataAnnotations;

namespace KeyProvider.Worker;

public sealed record KeyRotationOptions
{
    [Range(typeof(TimeSpan), "00:00:01", "30.00:00:00")]
    [Required]
    public required TimeSpan Interval { get; init; } = TimeSpan.FromMinutes(5);

    [Range(typeof(TimeSpan), "00:00:01", "30.00:00:00")]
    [Required]
    public required TimeSpan Timeout { get; init; } = TimeSpan.FromHours(2);
}
