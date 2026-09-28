using System.ComponentModel.DataAnnotations;

namespace KeyProvider.KeyRotation;

public sealed record KeyRotationOptions
{
    [Range(typeof(TimeSpan), "00:00:01", "30.00:00:00")]
    [Required]
    public required TimeSpan Lifespan { get; init; } = TimeSpan.FromHours(12);
}
