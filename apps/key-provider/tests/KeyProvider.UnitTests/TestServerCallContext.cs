using Grpc.Core;

namespace KeyProvider.UnitTests;

internal sealed class TestServerCallContext(CancellationToken cancellationToken) : ServerCallContext
{
    protected override string MethodCore => "/outpost.key.v1.KeyService/Test";
    protected override string HostCore => "localhost";
    protected override string PeerCore => "test";
    protected override DateTime DeadlineCore => DateTime.MaxValue;
    protected override Metadata RequestHeadersCore { get; } = new();
    protected override CancellationToken CancellationTokenCore => cancellationToken;
    protected override Metadata ResponseTrailersCore { get; } = new();
    protected override Status StatusCore { get; set; }
    protected override WriteOptions? WriteOptionsCore { get; set; }

    protected override AuthContext AuthContextCore { get; } =
        new(null, new Dictionary<string, List<AuthProperty>>());

    protected override ContextPropagationToken CreatePropagationTokenCore(ContextPropagationOptions? options)
    {
        throw new NotSupportedException();
    }

    protected override Task WriteResponseHeadersAsyncCore(Metadata responseHeaders)
    {
        return Task.CompletedTask;
    }
}
