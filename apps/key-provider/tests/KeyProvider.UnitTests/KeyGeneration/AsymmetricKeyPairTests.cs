using KeyProvider.KeyGeneration;

using Org.BouncyCastle.Crypto;
using Org.BouncyCastle.Crypto.Parameters;
using Org.BouncyCastle.Security;

using Shouldly;

namespace KeyProvider.UnitTests.KeyGeneration;

public sealed class AsymmetricKeyPairTests
{
    private static readonly Ed25519PrivateKeyParameters PrivateKey = new(new SecureRandom());
    private static readonly Ed25519PublicKeyParameters PublicKey = PrivateKey.GeneratePublicKey();

    [Fact]
    public void New_PublicKeyGivenAsThePrivateKey_Throws()
    {
        // Act
        ArgumentException error = Should.Throw<ArgumentException>(() =>
            new AsymmetricKeyPair<AsymmetricKeyParameter, AsymmetricKeyParameter>(PublicKey, PublicKey));

        // Assert
        error.ParamName.ShouldBe("PrivateKey");
        error.Message.ShouldStartWith("Expected a private key.");
    }

    [Fact]
    public void New_PrivateKeyGivenAsThePublicKey_Throws()
    {
        // Act
        ArgumentException error = Should.Throw<ArgumentException>(() =>
            new AsymmetricKeyPair<AsymmetricKeyParameter, AsymmetricKeyParameter>(PrivateKey, PrivateKey));

        // Assert
        error.ParamName.ShouldBe("PublicKey");
        error.Message.ShouldStartWith("Expected a public key.");
    }
}
