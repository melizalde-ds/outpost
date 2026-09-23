using Org.BouncyCastle.Crypto.Generators;
using Org.BouncyCastle.Crypto.Parameters;
using Org.BouncyCastle.Security;

namespace Key.Engine;

internal class ED25519 : IAsymmetricEngine<Ed25519PrivateKeyParameters, Ed25519PublicKeyParameters>
{
    private readonly Ed25519KeyPairGenerator _pairGenerator;
    private readonly ILogger _logger;

    public ED25519(ILogger logger)
    {
        this._pairGenerator = new Ed25519KeyPairGenerator();
        this._pairGenerator.Init(new Ed25519KeyGenerationParameters(new SecureRandom()));
        this._logger = logger;
    }

    public virtual AsymmetricKeyPair<Ed25519PrivateKeyParameters, Ed25519PublicKeyParameters> GenerateKeyPair()
    {
        var keyPair = this._pairGenerator.GenerateKeyPair();
        _logger.LogInformation("Generated ED25519 key pair");
        return new AsymmetricKeyPair<Ed25519PrivateKeyParameters, Ed25519PublicKeyParameters>(
            (Ed25519PrivateKeyParameters)keyPair.Private,
            (Ed25519PublicKeyParameters)keyPair.Public);
    }
}
