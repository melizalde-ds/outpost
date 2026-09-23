using Org.BouncyCastle.Crypto.Generators;
using Org.BouncyCastle.Crypto.Parameters;
using Org.BouncyCastle.Security;

namespace Key.Engine;

internal sealed class ED25519Engine : IAsymmetricEngine<Ed25519PrivateKeyParameters, Ed25519PublicKeyParameters>
{
    private readonly Ed25519KeyPairGenerator _pairGenerator;
    private readonly ILogger<ED25519Engine> _logger;

    public ED25519Engine(ILogger<ED25519Engine> logger)
    {
        this._pairGenerator = new Ed25519KeyPairGenerator();
        this._pairGenerator.Init(new Ed25519KeyGenerationParameters(new SecureRandom()));
        this._logger = logger;
    }

    public AsymmetricKeyPair<Ed25519PrivateKeyParameters, Ed25519PublicKeyParameters> GenerateKeyPair()
    {
        var keyPair = this._pairGenerator.GenerateKeyPair();
        _logger.LogInformation("Generated ED25519 key pair");
        return new AsymmetricKeyPair<Ed25519PrivateKeyParameters, Ed25519PublicKeyParameters>(
            (Ed25519PrivateKeyParameters)keyPair.Private,
            (Ed25519PublicKeyParameters)keyPair.Public);
    }
}
