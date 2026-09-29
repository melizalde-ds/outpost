namespace KeyProvider.Errors;

public sealed class SigningKeyNotFoundException()
    : Exception("No signing key is available, a key rotation has not completed yet.");
