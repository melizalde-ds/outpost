using System.ComponentModel.DataAnnotations;

namespace KeyProvider.Signing;

public sealed record SigningServiceOptions
{
    [Required]
    [Range(typeof(TimeSpan), "00:01:00", "00:15:00")]
    public required TimeSpan TokenLifetime { get; init; } = TimeSpan.FromMinutes(5);
}
