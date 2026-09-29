using Org.BouncyCastle.Crypto.Parameters;

namespace KeyProvider.KeyGeneration;

public sealed record SymmetricKey(KeyParameter Key) : CryptoKey;
