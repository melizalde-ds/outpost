using Org.BouncyCastle.Crypto;

namespace KeyProvider.KeyGeneration;

public interface IAsymmetricKeyGenerator<TPrivate, TPublic>
    where TPrivate : AsymmetricKeyParameter
    where TPublic : AsymmetricKeyParameter
{
    AsymmetricKeyPair<TPrivate, TPublic> GenerateKeyPair();
}
