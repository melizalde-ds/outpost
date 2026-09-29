using System.Text.Json.Serialization;

namespace KeyProvider.Signing;

public sealed record TokenHeader
{
    [JsonPropertyName("alg")] public required string Algorithm { get; init; }

    [JsonPropertyName("typ")] public required string Type { get; init; }

    [JsonPropertyName("kid")] public required Guid KeyId { get; init; }
}
