namespace KeyProvider.KeyPairHub;

public sealed record PrivateSigningKey
{
    public required Guid Id { get; init; }
    public required byte[] PrivateKey { get; init; }
    public required DateTimeOffset ExpirationTime { get; init; }
}
