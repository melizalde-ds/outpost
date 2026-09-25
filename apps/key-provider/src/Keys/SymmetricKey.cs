using Org.BouncyCastle.Crypto.Parameters;

namespace KeyProvider.Keys;

public sealed record SymmetricKey(KeyParameter Key) : CryptoKey;
