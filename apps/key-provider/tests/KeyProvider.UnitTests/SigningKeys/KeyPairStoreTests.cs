using KeyProvider.KeyRotation;
using KeyProvider.SigningKeys;

using Microsoft.AspNetCore.DataProtection;

using Org.BouncyCastle.Crypto.Parameters;

using Shouldly;

namespace KeyProvider.UnitTests.SigningKeys;

public sealed class KeyPairStoreTests
{
    private static readonly DateTimeOffset Now = new(2026, 1, 1, 0, 0, 0, TimeSpan.Zero);
    private static readonly Guid KeyId = new("00000000-0000-0000-0000-000000000001");

    private readonly IDataProtector _protector = new EphemeralDataProtectionProvider()
        .CreateProtector(KeyPairRecord.PrivateKeyProtectionPurpose);

    private static KeyPairRecord RecordWith(byte[] protectedPrivateKey)
    {
        KeyRotationJob job = new() { KeyPairRotationJobId = KeyId, StartedAt = Now, Success = true };
        return new KeyPairRecord
        {
            KeyPairId = KeyId,
            PublicKey = new byte[Ed25519PublicKeyParameters.KeySize],
            PrivateKey = protectedPrivateKey,
            ExpiresAt = Now,
            KeyPairRotationJobId = job.KeyPairRotationJobId,
            RotationJob = job
        };
    }

    [Fact]
    public void Decode_KeyProtectedWithTheStorePurpose_ReturnsThePrivateKey()
    {
        // Arrange
        byte[] privateKey = new byte[Ed25519PrivateKeyParameters.KeySize];
        privateKey[0] = 7;
        KeyPairRecord record = RecordWith(_protector.Protect(privateKey));

        // Act
        SigningKeyPair result = KeyPairStore.Decode(record, _protector);

        // Assert
        result.PrivateKey.PrivateKey.ShouldBe(privateKey);
    }

    [Fact]
    public void Decode_OneProtectedByteFlipped_ThrowsSigningKeyUndecryptable()
    {
        // Arrange
        byte[] protectedKey = _protector.Protect(new byte[Ed25519PrivateKeyParameters.KeySize]);
        protectedKey[^1] ^= 0x01;
        KeyPairRecord record = RecordWith(protectedKey);

        // Act
        Exception error = Should.Throw<Exception>(() => KeyPairStore.Decode(record, _protector));

        // Assert
        error.ShouldBeOfType<SigningKeyUndecryptableException>().KeyId.ShouldBe(KeyId);
    }

    [Fact]
    public void Decode_KeyProtectedForAnotherPurpose_ThrowsSigningKeyUndecryptable()
    {
        // Arrange
        IDataProtector otherProtector = _protector.CreateProtector("another purpose");
        KeyPairRecord record = RecordWith(otherProtector.Protect(new byte[Ed25519PrivateKeyParameters.KeySize]));

        // Act
        Exception error = Should.Throw<Exception>(() => KeyPairStore.Decode(record, _protector));

        // Assert
        error.ShouldBeOfType<SigningKeyUndecryptableException>();
    }

    [Fact]
    public void Decode_PrivateKeyOneByteTooShort_ThrowsSigningKeyMalformed()
    {
        // Arrange
        KeyPairRecord record = RecordWith(_protector.Protect(new byte[Ed25519PrivateKeyParameters.KeySize - 1]));

        // Act
        Exception error = Should.Throw<Exception>(() => KeyPairStore.Decode(record, _protector));

        // Assert
        error.ShouldBeOfType<SigningKeyMalformedException>().ActualLength.ShouldBe(31);
    }
}
