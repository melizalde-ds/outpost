using System.ComponentModel.DataAnnotations;

namespace KeyProvider.KeyPairServices;

public sealed class KeyRetrievalOptions
{
    [Required] public required TimeSpan Since { get; init; } = TimeSpan.FromDays(5);
}
