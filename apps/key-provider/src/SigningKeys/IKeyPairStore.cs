namespace KeyProvider.SigningKeys;

internal interface IKeyPairStore
{
    Task SaveKeyPairAsync(KeyPairRecord record, CancellationToken ct = default);

    Task<SigningKeyPair?> GetLatestKeyPairAsync(CancellationToken ct = default);

    Task<PublicSigningKey[]> GetVerificationKeysAsync(CancellationToken ct = default);
}
