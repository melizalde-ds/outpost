using Grpc.Core;
using Grpc.Core.Interceptors;

using KeyProvider.Database;
using KeyProvider.Signing;

namespace KeyProvider.Api;

internal sealed class ErrorInterceptor(ILogger<ErrorInterceptor> logger) : Interceptor
{
    public override async Task<TResponse> UnaryServerHandler<TRequest, TResponse>(TRequest request,
        ServerCallContext context, UnaryServerMethod<TRequest, TResponse> continuation)
    {
        try
        {
            return await continuation(request, context);
        }
        catch (RpcException)
        {
            throw;
        }
        catch (InvalidSubjectException e)
        {
            logger.LogInformation(e, "Rejecting {Method}: invalid subject", context.Method);
            throw new RpcException(new Status(StatusCode.InvalidArgument, "invalid subject"));
        }
        catch (Exception e) when (e is SigningKeyNotFoundException or SigningKeyExpiredException)
        {
            logger.LogWarning(e, "No usable signing key for {Method}", context.Method);
            throw new RpcException(new Status(StatusCode.Unavailable, "service unavailable, retry later"));
        }
        catch (DatabaseUnavailableException e)
        {
            logger.LogWarning(e, "Database temporarily unavailable for {Method}", context.Method);
            throw new RpcException(new Status(StatusCode.Unavailable, "service unavailable, retry later"));
        }
        catch (OperationCanceledException e) when (context.CancellationToken.IsCancellationRequested)
        {
            logger.LogInformation(e, "Client cancelled {Method}", context.Method);
            throw new RpcException(new Status(StatusCode.Cancelled, "request cancelled"));
        }
        catch (Exception e)
        {
            logger.LogError(e, "Internal failure in {Method}", context.Method);
            throw new RpcException(new Status(StatusCode.Internal, "internal error"));
        }
    }
}
