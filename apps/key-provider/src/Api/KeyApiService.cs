using Google.Protobuf;
using Google.Protobuf.WellKnownTypes;

using Grpc.Core;

using KeyProvider.Api.Generated.V1;
using KeyProvider.Signing;
using KeyProvider.SigningKeys;

namespace KeyProvider.Api;

internal sealed class KeyApiService(
    ILogger<KeyApiService> logger,
    SigningService service,
    IServiceScopeFactory scopeFactory) : KeyService.KeyServiceBase
{
    public override async Task<SignTokenResponse> SignToken(SignTokenRequest request, ServerCallContext context)
    {
        logger.LogInformation("Signing for user: {Subject}", request.Subject);
        if (!Guid.TryParse(request.Subject, out Guid subId))
        {
            throw new InvalidSubjectException(request.Subject);
        }

        SignedToken signed = await service.GenerateTokenAsync(subId, context.CancellationToken);

        SignTokenResponse response = new() { Token = signed.Token, KeyId = signed.KeyId.ToString() };

        if (signed.NewPublicKey is { } newPublicKey)
        {
            response.VerificationKey = new VerificationKey
            {
                KeyId = newPublicKey.Id.ToString(),
                PublicKey = ByteString.CopyFrom(newPublicKey.PublicKey),
                ExpiresAt = newPublicKey.ExpirationTime.ToTimestamp()
            };
        }

        return response;
    }

    public override async Task<GetVerificationKeysResponse> GetVerificationKeys(GetVerificationKeysRequest request,
        ServerCallContext context)
    {
        logger.LogInformation("Getting verification keys...");
        await using AsyncServiceScope scope = scopeFactory.CreateAsyncScope();
        KeyPairStore keyPairStore = scope.ServiceProvider.GetRequiredService<KeyPairStore>();

        PublicSigningKey[] sourceKeys = await keyPairStore.GetVerificationKeysAsync(context.CancellationToken);

        List<VerificationKey> publicKeys = new();

        publicKeys.AddRange(sourceKeys.Select(k => new VerificationKey
        {
            KeyId = k.Id.ToString(),
            PublicKey = ByteString.CopyFrom(k.PublicKey),
            ExpiresAt = k.ExpirationTime.ToTimestamp()
        }));
        logger.LogInformation("Sending {PublicKeyCount} verification keys...", publicKeys.Count);
        return new GetVerificationKeysResponse { Keys = { publicKeys } };
    }
}
