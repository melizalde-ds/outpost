using Org.BouncyCastle.Crypto;

namespace Key.Engine;

public interface IAsymmetricEngine<TPrivate, TPublic>
    where TPrivate : AsymmetricKeyParameter
    where TPublic : AsymmetricKeyParameter
{
    AsymmetricKeyPair<TPrivate, TPublic> GenerateKeyPair();

    IEnumerable<TPublic> GetPublicKeys(TPrivate privateKey);
}
