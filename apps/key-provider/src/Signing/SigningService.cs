using System.Buffers.Text;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;

using KeyProvider.SigningKeys;

using Microsoft.Extensions.Options;

using Org.BouncyCastle.Crypto.Parameters;
using Org.BouncyCastle.Crypto.Signers;

namespace KeyProvider.Signing;

internal sealed class SigningService(
    IOptions<SigningOptions> options,
    IServiceScopeFactory scopeFactory,
    TimeProvider timeProvider) : IDisposable
{
    private readonly SigningOptions _options = options.Value;
    private readonly SemaphoreSlim _refreshLock = new(1, 1);
    private volatile PrivateSigningKey? _activePrivateKey;

    public void Dispose()
    {
        if (_activePrivateKey is { } key)
        {
            CryptographicOperations.ZeroMemory(key.PrivateKey);
            _activePrivateKey = null;
        }

        _refreshLock.Dispose();
    }

    private byte[] GetHeader(PrivateSigningKey key)
    {
        TokenHeader header = new() { Algorithm = _options.Algorithm, Type = _options.Type, KeyId = key.Id };

        return JsonSerializer.SerializeToUtf8Bytes(header);
    }

    private byte[] GetPayload(Guid sub)
    {
        DateTimeOffset now = timeProvider.GetUtcNow();
        Dictionary<string, object> claims = new()
        {
            ["iss"] = _options.Issuer,
            ["sub"] = sub.ToString(),
            ["iat"] = now.ToUnixTimeSeconds(),
            ["exp"] = (now + _options.TokenLifetime).ToUnixTimeSeconds(),
            ["jti"] = Guid.CreateVersion7(now).ToString("N")
        };

        return JsonSerializer.SerializeToUtf8Bytes(claims);
    }

    internal async Task<(string Token, PublicSigningKey? PublicKey)> GenerateTokenAsync(Guid sub,
        CancellationToken ct = default)
    {
        (PrivateSigningKey privateKey, PublicSigningKey? publicKey) = await GetKeyPairAsync(ct);
        byte[] headerJson = GetHeader(privateKey);
        byte[] payloadJson = GetPayload(sub);

        string signingInput = $"{B64Url(headerJson)}.{B64Url(payloadJson)}";
        byte[] data = Encoding.ASCII.GetBytes(signingInput);

        Ed25519Signer signer = new();
        signer.Init(true, new Ed25519PrivateKeyParameters(privateKey.PrivateKey));
        signer.BlockUpdate(data, 0, data.Length);

        byte[] signature = signer.GenerateSignature();

        return ($"{signingInput}.{B64Url(signature)}", publicKey);
    }

    private async Task<(PrivateSigningKey PrivateKey, PublicSigningKey? PublicKey)> GetKeyPairAsync(
        CancellationToken ct)
    {
        PrivateSigningKey? cached = _activePrivateKey;
        if (cached is not null && CanSign(cached))
        {
            return (cached, null);
        }

        await _refreshLock.WaitAsync(ct);
        try
        {
            cached = _activePrivateKey;
            if (cached is not null && CanSign(cached))
            {
                return (cached, null);
            }

            await using AsyncServiceScope scope = scopeFactory.CreateAsyncScope();
            KeyPairStore retrieval = scope.ServiceProvider.GetRequiredService<KeyPairStore>();

            SigningKeyPair fresh = await retrieval.GetLatestKeyPairAsync(ct)
                                   ?? throw new SigningKeyNotFoundException();

            if (!CanSign(fresh.PrivateKey))
            {
                throw new SigningKeyExpiredException(fresh.PrivateKey.Id, fresh.PrivateKey.ExpirationTime);
            }

            _activePrivateKey = fresh.PrivateKey;
            return (fresh.PrivateKey, fresh.PublicKey);
        }
        finally
        {
            _refreshLock.Release();
        }
    }

    private bool CanSign(PrivateSigningKey key)
    {
        return key.ExpirationTime > timeProvider.GetUtcNow() + _options.TokenLifetime;
    }

    private static string B64Url(ReadOnlySpan<byte> b)
    {
        return Base64Url.EncodeToString(b);
    }
}
