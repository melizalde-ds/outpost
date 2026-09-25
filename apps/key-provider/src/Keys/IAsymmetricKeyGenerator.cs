using Org.BouncyCastle.Crypto;

namespace KeyProvider.Keys;

public interface IAsymmetricKeyGenerator<TPrivate, TPublic>
    where TPrivate : AsymmetricKeyParameter
    where TPublic : AsymmetricKeyParameter
{
    AsymmetricKeyPair<TPrivate, TPublic> GenerateKeyPair();
}
