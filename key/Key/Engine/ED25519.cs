using Org.BouncyCastle.Crypto.Generators;
using Org.BouncyCastle.Crypto.Parameters;
using Org.BouncyCastle.Security;

namespace Key.Engine;

public class ED25519 : IAsymmetricEngine<Ed25519PrivateKeyParameters, Ed25519PublicKeyParameters>
{
    private readonly Ed25519KeyPairGenerator _pairGenerator;

    public ED25519()
    {
        this._pairGenerator = new Ed25519KeyPairGenerator();
        this._pairGenerator.Init(new Ed25519KeyGenerationParameters(new SecureRandom()));
    }

    public AsymmetricKeyPair<Ed25519PrivateKeyParameters, Ed25519PublicKeyParameters> GenerateKeyPair()
    {
        throw new NotImplementedException();
    }

    public IEnumerable<Ed25519PublicKeyParameters> GetPublicKeys(Ed25519PrivateKeyParameters privateKey)
    {
        throw new NotImplementedException();
    }
}
