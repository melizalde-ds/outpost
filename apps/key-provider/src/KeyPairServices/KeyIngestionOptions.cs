using System.ComponentModel.DataAnnotations;

namespace KeyProvider.KeyPairServices;

public sealed class KeyIngestionOptions
{
    [Range(typeof(TimeSpan), "00:00:01", "30.00:00:00")]
    [Required]
    public required TimeSpan Interval { get; init; } = TimeSpan.FromMinutes(30);
}
