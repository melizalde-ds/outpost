using Org.BouncyCastle.Crypto;
using Org.BouncyCastle.Crypto.Parameters;

namespace Key.Engine;

public abstract record CryptoKey;

public sealed record SymmetricKey(KeyParameter Key) : CryptoKey;

public record AsymmetricKeyPair<TPrivate, TPublic>(TPrivate PrivateKey, TPublic PublicKey) : CryptoKey
    where TPrivate : AsymmetricKeyParameter
    where TPublic : AsymmetricKeyParameter
{
    public TPrivate PrivateKey { get; init; } = PrivateKey.IsPrivate
        ? PrivateKey
        : throw new ArgumentException("Expected a private key.", nameof(PrivateKey));

    public TPublic PublicKey { get; init; } = !PublicKey.IsPrivate
        ? PublicKey
        : throw new ArgumentException("Expected a public key.", nameof(PublicKey));
}

