using Org.BouncyCastle.Crypto;

namespace KeyProvider.KeyGeneration;

public sealed record AsymmetricKeyPair<TPrivate, TPublic>(TPrivate PrivateKey, TPublic PublicKey) : CryptoKey
    where TPrivate : AsymmetricKeyParameter
    where TPublic : AsymmetricKeyParameter
{
    public TPrivate PrivateKey { get; } = PrivateKey.IsPrivate
        ? PrivateKey
        : throw new ArgumentException("Expected a private key.", nameof(PrivateKey));

    public TPublic PublicKey { get; } = !PublicKey.IsPrivate
        ? PublicKey
        : throw new ArgumentException("Expected a public key.", nameof(PublicKey));
}
