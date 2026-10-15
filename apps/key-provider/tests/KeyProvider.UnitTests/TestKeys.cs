using System.Buffers.Text;
using System.Text;
using System.Text.Json;

using KeyProvider.KeyGeneration;
using KeyProvider.SigningKeys;

using Microsoft.Extensions.Logging.Abstractions;

using Org.BouncyCastle.Crypto.Parameters;
using Org.BouncyCastle.Crypto.Signers;

namespace KeyProvider.UnitTests;

internal static class TestKeys
{
    private static readonly Ed25519KeyGenerator Generator = new(NullLogger<Ed25519KeyGenerator>.Instance);

    internal static SigningKeyPair Create(Guid id, DateTimeOffset expiresAt)
    {
        AsymmetricKeyPair<Ed25519PrivateKeyParameters, Ed25519PublicKeyParameters> pair =
            Generator.GenerateKeyPair();
        return new SigningKeyPair(
            new PrivateSigningKey { Id = id, PrivateKey = pair.PrivateKey.GetEncoded(), ExpirationTime = expiresAt },
            new PublicSigningKey { Id = id, PublicKey = pair.PublicKey.GetEncoded(), ExpirationTime = expiresAt });
    }

    internal static bool Verify(string token, byte[] publicKey)
    {
        int lastDot = token.LastIndexOf('.');
        byte[] signedPart = Encoding.ASCII.GetBytes(token, 0, lastDot);
        byte[] signature = Base64Url.DecodeFromChars(token.AsSpan(lastDot + 1));

        Ed25519Signer verifier = new();
        verifier.Init(false, new Ed25519PublicKeyParameters(publicKey));
        verifier.BlockUpdate(signedPart, 0, signedPart.Length);
        return verifier.VerifySignature(signature);
    }

    internal static string WithPayload(string token, string payloadJson)
    {
        string[] parts = token.Split('.');
        return $"{parts[0]}.{Base64Url.EncodeToString(Encoding.UTF8.GetBytes(payloadJson))}.{parts[2]}";
    }

    internal static JsonElement Header(string token)
    {
        return Decode(token.Split('.')[0]);
    }

    internal static JsonElement Payload(string token)
    {
        return Decode(token.Split('.')[1]);
    }

    private static JsonElement Decode(string part)
    {
        return JsonSerializer.Deserialize<JsonElement>(Base64Url.DecodeFromChars(part));
    }
}
