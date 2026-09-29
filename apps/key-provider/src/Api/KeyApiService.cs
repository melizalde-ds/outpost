using Google.Protobuf;
using Google.Protobuf.WellKnownTypes;

using Grpc.Core;

using KeyProvider.Api.Generated.V1;
using KeyProvider.Signing;
using KeyProvider.SigningKeys;

namespace KeyProvider.Api;

internal sealed class KeyApiService(
    ILogger<KeyApiService> logger,
    SigningService service) : KeyService.KeyServiceBase
{
    public override async Task<SignTokenResponse> SignToken(SignTokenRequest request, ServerCallContext context)
    {
        logger.LogInformation("Signing for user: {Subject}", request.Subject);
        if (string.IsNullOrEmpty(request.Subject))
        {
            throw new ArgumentException("Subject cannot be null or empty");
        }

        if (!Guid.TryParse(request.Subject, out Guid subId))
        {
            throw new ArgumentException($"Sub {request.Subject} is not a valid GUID");
        }

        (string token, PublicSigningKey? publicKey) = await service.GenerateTokenAsync(subId);

        if (publicKey is not null)
        {
            VerificationKey key = new()
            {
                KeyId = publicKey.Id.ToString(),
                PublicKey = ByteString.CopyFrom(publicKey.PublicKey),
                ExpiresAt = publicKey.ExpirationTime.ToTimestamp()
            };

            return new SignTokenResponse { Token = token, VerificationKey = key };
        }

        return new SignTokenResponse { Token = token };
    }

    public override Task<GetVerificationKeysResponse> GetVerificationKeys(GetVerificationKeysRequest request,
        ServerCallContext context)
    {
        return base.GetVerificationKeys(request, context);
    }
}
