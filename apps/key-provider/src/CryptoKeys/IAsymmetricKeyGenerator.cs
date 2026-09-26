using Org.BouncyCastle.Crypto;

namespace KeyProvider.CryptoKeys;

public interface IAsymmetricKeyGenerator<TPrivate, TPublic>
    where TPrivate : AsymmetricKeyParameter
    where TPublic : AsymmetricKeyParameter
{
    AsymmetricKeyPair<TPrivate, TPublic> GenerateKeyPair();
}
