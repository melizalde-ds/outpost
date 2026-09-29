namespace KeyProvider.SigningKeys;

public sealed record PublicSigningKey
{
    public required Guid Id { get; init; }
    public required byte[] PublicKey { get; init; }
    public required DateTimeOffset ExpirationTime { get; init; }
}
