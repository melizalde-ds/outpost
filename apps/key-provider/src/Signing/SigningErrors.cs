namespace KeyProvider.Signing;

public abstract class SigningKeyException(string message) : Exception(message);

public sealed class SigningKeyNotFoundException()
    : SigningKeyException("No signing key is available, a key rotation has not completed yet.");

public sealed class SigningKeyExpiredException(Guid keyId, DateTimeOffset expiresAt)
    : SigningKeyException($"Signing key '{keyId}' expires at {expiresAt:O} and can no longer sign tokens.")
{
    public Guid KeyId { get; } = keyId;
    public DateTimeOffset ExpiresAt { get; } = expiresAt;
}
