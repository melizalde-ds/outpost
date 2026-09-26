using System.ComponentModel.DataAnnotations;

namespace KeyProvider.KeyPairServices;

public sealed class KeyIngestionOptions
{
    [Required] public required TimeSpan Interval { get; init; } = TimeSpan.FromMinutes(30);
}
