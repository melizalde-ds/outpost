using System.ComponentModel.DataAnnotations;

namespace KeyProvider.KeyPairServices;

public sealed class KeyRetrievalOptions
{
    [Range(typeof(TimeSpan), "00:00:01", "30.00:00:00")]
    [Required]
    public required TimeSpan Since { get; init; } = TimeSpan.FromDays(5);
}
