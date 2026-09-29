namespace KeyProvider.SigningKeys;

public sealed class SigningKeyUndecryptableException(Guid keyId, Exception innerException)
    : Exception($"Signing key '{keyId}' could not be decrypted.", innerException)
{
    public Guid KeyId { get; } = keyId;
}

public sealed class SigningKeyMalformedException(Guid keyId, int actualLength, int expectedLength)
    : Exception($"Signing key '{keyId}' is {actualLength} bytes long, expected {expectedLength}.")
{
    public Guid KeyId { get; } = keyId;
    public int ActualLength { get; } = actualLength;
    public int ExpectedLength { get; } = expectedLength;
}
