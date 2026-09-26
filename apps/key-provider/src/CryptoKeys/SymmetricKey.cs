using Org.BouncyCastle.Crypto.Parameters;

namespace KeyProvider.CryptoKeys;

public sealed record SymmetricKey(KeyParameter Key) : CryptoKey;
