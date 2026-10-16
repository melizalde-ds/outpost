using System.Text.Json;
using Google.Protobuf.WellKnownTypes;
using Grpc.Core;
using KeyProvider.IntegrationTests.Infrastructure;
using KeyProvider.UnitTests;
using Outpost.Key.V1;
using Shouldly;

namespace KeyProvider.IntegrationTests.Api;

[Trait("Consumer", "auth")]
public sealed class SignTokenTests(PostgresFixture postgres)
{
    private const string Subject = "00000000-0000-0000-0000-000000000001";
    private static readonly TimeSpan KeyLifespan = TimeSpan.FromDays(1);
    private static readonly TimeSpan TokenLifetime = TimeSpan.FromMinutes(5);

    private readonly CancellationToken _ct = TestContext.Current.CancellationToken;

    private async Task<ServiceFactory> StartAsync()
    {
        return new ServiceFactory(await postgres.CreateDatabasesAsync(_ct));
    }

    private Task<SignTokenResponse> SignAsync(ServiceFactory service, string subject = Subject)
    {
        return service
            .CreateKeyClient()
            .SignTokenAsync(new SignTokenRequest { Subject = subject }, cancellationToken: _ct)
            .ResponseAsync;
    }

    [Fact]
    public async Task SignToken_FirstCallAfterRotation_TokenVerifiesWithTheReturnedKey()
    {
        // Arrange
        await using ServiceFactory service = await StartAsync();
        Guid keyId = await service.RotateAsync(_ct);

        // Act
        SignTokenResponse response = await SignAsync(service);

        // Assert
        TestKeys
            .Verify(response.Token, response.VerificationKey.PublicKey.ToByteArray())
            .ShouldBeTrue();
        response.KeyId.ShouldBe(keyId.ToString());
        response.VerificationKey.KeyId.ShouldBe(keyId.ToString());
        response.VerificationKey.ExpiresAt.ShouldBe(
            Timestamp.FromDateTimeOffset(ServiceFactory.Start + KeyLifespan)
        );
    }

    [Fact]
    public async Task SignToken_ValidSubject_TokenNamesTheUserTheKeyAndTheExpiry()
    {
        // Arrange
        await using ServiceFactory service = await StartAsync();
        await service.RotateAsync(_ct);

        // Act
        SignTokenResponse response = await SignAsync(service);

        // Assert
        JsonElement header = TestKeys.Header(response.Token);
        JsonElement payload = TestKeys.Payload(response.Token);
        header.GetProperty("alg").GetString().ShouldBe("EdDSA");
        header.GetProperty("kid").GetString().ShouldBe(response.KeyId);
        payload.GetProperty("sub").GetString().ShouldBe(Subject);
        payload.GetProperty("iss").GetString().ShouldBe("Outpost");
        payload.GetProperty("aud").GetString().ShouldBe("Outpost");
        payload.GetProperty("iat").GetInt64().ShouldBe(ServiceFactory.Start.ToUnixTimeSeconds());
        payload
            .GetProperty("exp")
            .GetInt64()
            .ShouldBe((ServiceFactory.Start + TokenLifetime).ToUnixTimeSeconds());
    }

    [Fact]
    public async Task SignToken_SecondCall_DoesNotSendTheKeyAgain()
    {
        // Arrange
        await using ServiceFactory service = await StartAsync();
        await service.RotateAsync(_ct);
        await SignAsync(service);

        // Act
        SignTokenResponse response = await SignAsync(service);

        // Assert
        response.VerificationKey.ShouldBeNull();
    }

    [Fact]
    public async Task SignToken_NewerKeyWhileCurrentKeyIsGood_KeepsSigningWithTheCurrentKey()
    {
        // Arrange
        await using ServiceFactory service = await StartAsync();
        Guid current = await service.RotateAsync(_ct);
        await SignAsync(service);
        service.Time.Advance(TimeSpan.FromHours(1));
        await service.RotateAsync(_ct);

        // Act
        SignTokenResponse response = await SignAsync(service);

        // Assert
        response.KeyId.ShouldBe(current.ToString());
        response.VerificationKey.ShouldBeNull();
    }

    [Fact]
    public async Task SignToken_CurrentKeyEndsWithinATokenLifetime_SwitchesToTheNewerKeyAndSendsIt()
    {
        // Arrange
        await using ServiceFactory service = await StartAsync();
        await service.RotateAsync(_ct);
        await SignAsync(service);
        service.Time.Advance(TimeSpan.FromHours(1));
        Guid newer = await service.RotateAsync(_ct);
        service.Time.SetUtcNow(ServiceFactory.Start + KeyLifespan - TokenLifetime);

        // Act
        SignTokenResponse response = await SignAsync(service);

        // Assert
        response.KeyId.ShouldBe(newer.ToString());
        response.VerificationKey.KeyId.ShouldBe(newer.ToString());
        TestKeys
            .Verify(response.Token, response.VerificationKey.PublicKey.ToByteArray())
            .ShouldBeTrue();
    }

    [Fact]
    public async Task SignToken_OnlyKeyEndsExactlyOneTokenLifetimeFromNow_ReturnsUnavailable()
    {
        // Arrange
        await using ServiceFactory service = await StartAsync();
        await service.RotateAsync(_ct);
        service.Time.SetUtcNow(ServiceFactory.Start + KeyLifespan - TokenLifetime);

        // Act
        RpcException error = await Should.ThrowAsync<RpcException>(() => SignAsync(service));

        // Assert
        error.StatusCode.ShouldBe(StatusCode.Unavailable);
    }

    [Fact]
    public async Task SignToken_NoKeyYet_ReturnsUnavailable()
    {
        // Arrange
        await using ServiceFactory service = await StartAsync();

        // Act
        RpcException error = await Should.ThrowAsync<RpcException>(() => SignAsync(service));

        // Assert
        error.StatusCode.ShouldBe(StatusCode.Unavailable);
    }

    [Fact]
    public async Task SignToken_SubjectNotAUuid_ReturnsInvalidArgument()
    {
        // Arrange
        await using ServiceFactory service = await StartAsync();
        await service.RotateAsync(_ct);

        // Act
        RpcException error = await Should.ThrowAsync<RpcException>(() =>
            SignAsync(service, "alice")
        );

        // Assert
        error.StatusCode.ShouldBe(StatusCode.InvalidArgument);
    }
}
