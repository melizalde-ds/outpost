using System.ComponentModel.DataAnnotations;

namespace KeyProvider.KeyRotation;

public sealed record KeyRotationOptions
{
    [Range(typeof(TimeSpan), "00:05:00", "30.00:00:00")]
    public TimeSpan Lifespan { get; init; } = TimeSpan.FromHours(12);
}
