using Org.BouncyCastle.Crypto.Generators;
using Org.BouncyCastle.Crypto.Parameters;
using Org.BouncyCastle.Security;

namespace KeyProvider.Keys;

internal sealed class Ed25519KeyGenerator : IAsymmetricKeyGenerator<Ed25519PrivateKeyParameters, Ed25519PublicKeyParameters>
{
    private readonly Ed25519KeyPairGenerator _pairGenerator;
    private readonly ILogger<Ed25519KeyGenerator> _logger;

    public Ed25519KeyGenerator(ILogger<Ed25519KeyGenerator> logger)
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
