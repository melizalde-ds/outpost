using Grpc.Core;

using KeyProvider.Api.Generated.V1;

namespace KeyProvider.Api;

internal sealed class KeyApiService(
    ILogger<KeyApiService> logger,
    IServiceScopeFactory scopeFactory) : KeyService.KeyServiceBase
{
    public override async Task<SignTokenResponse> SignToken(SignTokenRequest request, ServerCallContext context)
    {
        logger.LogInformation("Signing for user: {ID}", request.Subject);
        await using AsyncServiceScope scope = scopeFactory.CreateAsyncScope();

        throw new NotImplementedException();
    }

    public override Task<GetVerificationKeysResponse> GetVerificationKeys(GetVerificationKeysRequest request,
        ServerCallContext context)
    {
        return base.GetVerificationKeys(request, context);
    }

    public override Task<GetVerificationKeyResponse> GetVerificationKey(GetVerificationKeyRequest request,
        ServerCallContext context)
    {
        return base.GetVerificationKey(request, context);
    }
}
