namespace KeyProvider.Errors;

public sealed class SigningKeyExpiredException(Guid keyId, DateTimeOffset expiresAt)
    : Exception($"Signing key '{keyId}' expires at {expiresAt:O} and can no longer sign tokens.")
{
    public Guid KeyId { get; } = keyId;
    public DateTimeOffset ExpiresAt { get; } = expiresAt;
}
