using Grpc.Core;

using KeyProvider.Api;

using Microsoft.Extensions.Logging.Abstractions;

using Shouldly;

namespace KeyProvider.UnitTests.Api;

public sealed class ErrorInterceptorTests
{
    private readonly ErrorInterceptor _sut = new(NullLogger<ErrorInterceptor>.Instance);

    private Task<string> HandleAsync(Exception error, CancellationToken ct)
    {
        return _sut.UnaryServerHandler<string, string>(
            "request",
            new TestServerCallContext(ct),
            (_, _) => Task.FromException<string>(error));
    }

    [Fact]
    public async Task UnaryServerHandler_RpcException_PassesItThroughUnchanged()
    {
        // Arrange
        RpcException original = new(new Status(StatusCode.NotFound, "kept"));

        // Act
        RpcException error = await Should.ThrowAsync<RpcException>(() =>
            HandleAsync(original, TestContext.Current.CancellationToken));

        // Assert
        error.ShouldBeSameAs(original);
    }

    [Fact]
    public async Task UnaryServerHandler_InvalidSubject_ReturnsInvalidArgumentWithoutTheSubject()
    {
        // Act
        RpcException error = await Should.ThrowAsync<RpcException>(() =>
            HandleAsync(new InvalidSubjectException("private-subject"), TestContext.Current.CancellationToken));

        // Assert
        error.Status.ShouldBe(new Status(StatusCode.InvalidArgument, "invalid subject"));
    }

    [Fact]
    public async Task UnaryServerHandler_ClientCancelled_ReturnsCancelled()
    {
        // Arrange
        using CancellationTokenSource cancellation = new();
        await cancellation.CancelAsync();

        // Act
        RpcException error = await Should.ThrowAsync<RpcException>(() =>
            HandleAsync(new OperationCanceledException(cancellation.Token), cancellation.Token));

        // Assert
        error.Status.ShouldBe(new Status(StatusCode.Cancelled, "request cancelled"));
    }

    [Fact]
    public async Task UnaryServerHandler_UnexpectedError_ReturnsInternalError()
    {
        // Act
        RpcException error = await Should.ThrowAsync<RpcException>(() =>
            HandleAsync(new InvalidOperationException("boom"), TestContext.Current.CancellationToken));

        // Assert
        error.Status.ShouldBe(new Status(StatusCode.Internal, "internal error"));
    }

    [Fact]
    public void InvalidSubjectException_Message_NamesTheSubject()
    {
        // Act
        InvalidSubjectException error = new("not-a-user-id");

        // Assert
        error.Message.ShouldBe("Subject 'not-a-user-id' is not a valid user ID.");
    }
}
