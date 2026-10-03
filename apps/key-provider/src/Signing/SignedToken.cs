using KeyProvider.SigningKeys;

namespace KeyProvider.Signing;

internal sealed record SignedToken(string Token, Guid KeyId, PublicSigningKey? NewPublicKey);
