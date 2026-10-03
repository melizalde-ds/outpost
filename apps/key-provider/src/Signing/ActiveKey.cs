using KeyProvider.SigningKeys;

namespace KeyProvider.Signing;

internal sealed record ActiveKey(PrivateSigningKey PrivateKey, PublicSigningKey? NewPublicKey);
