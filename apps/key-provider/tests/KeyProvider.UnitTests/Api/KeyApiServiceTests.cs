using System.Text.Json;

using Grpc.Core;

using KeyProvider.Api;
using KeyProvider.Database;
using KeyProvider.Signing;
using KeyProvider.SigningKeys;

using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging.Abstractions;
using Microsoft.Extensions.Options;
using Microsoft.Extensions.Time.Testing;

using NSubstitute;
using NSubstitute.ExceptionExtensions;

using Outpost.Key.V1;

using Shouldly;

namespace KeyProvider.UnitTests.Api;

public sealed class KeyApiServiceTests : IDisposable
{
    private static readonly DateTimeOffset Now = new(2026, 1, 1, 0, 0, 0, TimeSpan.Zero);
    private static readonly TimeSpan TokenLifetime = TimeSpan.FromMinutes(5);
    private static readonly Guid UserId = Guid.Parse("00000000-0000-0000-0000-000000000001");
    private static readonly Guid KeyId = Guid.Parse("00000000-0000-0000-0000-0000000000a1");
    private static readonly Guid NewerKeyId = Guid.Parse("00000000-0000-0000-0000-0000000000a2");

    private readonly FakeTimeProvider _clock = new(Now);
    private readonly IKeyPairStore _store = Substitute.For<IKeyPairStore>();
    private readonly ServiceProvider _services;
    private readonly SigningService _signing;
    private readonly KeyApiService _api;
    private readonly ErrorInterceptor _interceptor = new(NullLogger<ErrorInterceptor>.Instance);

    public KeyApiServiceTests()
    {
        _services = new ServiceCollection().AddScoped(_ => _store).BuildServiceProvider();
        IServiceScopeFactory scopes = _services.GetRequiredService<IServiceScopeFactory>();
        _signing = new SigningService(Options.Create(new SigningOptions()), scopes, _clock);
        _api = new KeyApiService(NullLogger<KeyApiService>.Instance, _signing, scopes);
    }

    public void Dispose()
    {
        _signing.Dispose();
        _services.Dispose();
    }

    private SigningKeyPair LatestKeyIs(Guid id, DateTimeOffset expiresAt)
    {
        SigningKeyPair keys = TestKeys.Create(id, expiresAt);
        _store.GetLatestKeyPairAsync(Arg.Any<CancellationToken>()).Returns(keys);
        return keys;
    }

    private Task<SignTokenResponse> SignTokenAsync(string subject)
    {
        return _interceptor.UnaryServerHandler(
            new SignTokenRequest { Subject = subject },
            new TestServerCallContext(TestContext.Current.CancellationToken),
            (request, context) => _api.SignToken(request, context));
    }

    private Task<GetVerificationKeysResponse> GetVerificationKeysAsync()
    {
        return _interceptor.UnaryServerHandler(
            new GetVerificationKeysRequest(),
            new TestServerCallContext(TestContext.Current.CancellationToken),
            (request, context) => _api.GetVerificationKeys(request, context));
    }

    [Fact]
    public async Task SignToken_ValidSubject_ReturnsTokenSignedByTheLatestKey()
    {
        // Arrange
        SigningKeyPair keys = LatestKeyIs(KeyId, Now.AddDays(1));

        // Act
        SignTokenResponse response = await SignTokenAsync(UserId.ToString());

        // Assert
        TestKeys.Verify(response.Token, keys.PublicKey.PublicKey).ShouldBeTrue();
        response.KeyId.ShouldBe(KeyId.ToString());
    }

    [Fact]
    public async Task SignToken_TokenChangedAfterSigning_FailsVerification()
    {
        // Arrange
        SigningKeyPair keys = LatestKeyIs(KeyId, Now.AddDays(1));
        SignTokenResponse response = await SignTokenAsync(UserId.ToString());
        string tampered = TestKeys.WithPayload(response.Token, """{"sub":"someone-else"}""");

        // Act
        bool valid = TestKeys.Verify(tampered, keys.PublicKey.PublicKey);

        // Assert
        valid.ShouldBeFalse();
    }

    [Fact]
    public async Task SignToken_ValidSubject_HeaderNamesTheAlgorithmTypeAndKey()
    {
        // Arrange
        LatestKeyIs(KeyId, Now.AddDays(1));

        // Act
        SignTokenResponse response = await SignTokenAsync(UserId.ToString());

        // Assert
        JsonElement header = TestKeys.Header(response.Token);
        header.GetProperty("alg").GetString().ShouldBe("EdDSA");
        header.GetProperty("typ").GetString().ShouldBe("JWT");
        header.GetProperty("kid").GetString().ShouldBe(KeyId.ToString());
    }

    [Fact]
    public async Task SignToken_ValidSubject_ClaimsNameTheUserAndExpireAfterTheTokenLifetime()
    {
        // Arrange
        LatestKeyIs(KeyId, Now.AddDays(1));

        // Act
        SignTokenResponse response = await SignTokenAsync(UserId.ToString());

        // Assert
        JsonElement payload = TestKeys.Payload(response.Token);
        payload.GetProperty("sub").GetString().ShouldBe(UserId.ToString());
        payload.GetProperty("iss").GetString().ShouldBe("Outpost");
        payload.GetProperty("aud").GetString().ShouldBe("Outpost");
        payload.GetProperty("iat").GetInt64().ShouldBe(Now.ToUnixTimeSeconds());
        payload.GetProperty("exp").GetInt64().ShouldBe(Now.Add(TokenLifetime).ToUnixTimeSeconds());
        payload.GetProperty("jti").GetString().ShouldNotBeNullOrEmpty();
    }

    [Fact]
    public async Task SignToken_FirstTokenWithAKey_ReturnsThePublicKey()
    {
        // Arrange
        SigningKeyPair keys = LatestKeyIs(KeyId, Now.AddDays(1));

        // Act
        SignTokenResponse response = await SignTokenAsync(UserId.ToString());

        // Assert
        response.VerificationKey.KeyId.ShouldBe(KeyId.ToString());
        response.VerificationKey.PublicKey.ToByteArray().ShouldBe(keys.PublicKey.PublicKey);
        response.VerificationKey.ExpiresAt.ToDateTimeOffset().ShouldBe(Now.AddDays(1));
    }

    [Fact]
    public async Task SignToken_SecondTokenWithTheSameKey_ReturnsNoPublicKey()
    {
        // Arrange
        LatestKeyIs(KeyId, Now.AddDays(1));
        await SignTokenAsync(UserId.ToString());

        // Act
        SignTokenResponse response = await SignTokenAsync(UserId.ToString());

        // Assert
        response.VerificationKey.ShouldBeNull();
    }

    [Fact]
    public async Task SignToken_KeyOutlivesATokenLifetime_KeepsUsingTheKey()
    {
        // Arrange
        DateTimeOffset expiresAt = Now.AddHours(1);
        LatestKeyIs(KeyId, expiresAt);
        await SignTokenAsync(UserId.ToString());
        LatestKeyIs(NewerKeyId, Now.AddDays(1));
        _clock.SetUtcNow(expiresAt - TokenLifetime - TimeSpan.FromSeconds(1));

        // Act
        SignTokenResponse response = await SignTokenAsync(UserId.ToString());

        // Assert
        response.KeyId.ShouldBe(KeyId.ToString());
    }

    [Fact]
    public async Task SignToken_KeyExpiresWithinATokenLifetime_SwitchesToTheNewestKey()
    {
        // Arrange
        DateTimeOffset expiresAt = Now.AddHours(1);
        LatestKeyIs(KeyId, expiresAt);
        await SignTokenAsync(UserId.ToString());
        LatestKeyIs(NewerKeyId, Now.AddDays(1));
        _clock.SetUtcNow(expiresAt - TokenLifetime);

        // Act
        SignTokenResponse response = await SignTokenAsync(UserId.ToString());

        // Assert
        response.KeyId.ShouldBe(NewerKeyId.ToString());
        response.VerificationKey.KeyId.ShouldBe(NewerKeyId.ToString());
    }

    [Theory]
    [InlineData("")]
    [InlineData("not-a-user-id")]
    public async Task SignToken_InvalidSubject_ReturnsInvalidArgument(string subject)
    {
        // Arrange
        LatestKeyIs(KeyId, Now.AddDays(1));

        // Act
        RpcException error = await Should.ThrowAsync<RpcException>(() => SignTokenAsync(subject));

        // Assert
        error.StatusCode.ShouldBe(StatusCode.InvalidArgument);
    }

    [Fact]
    public async Task SignToken_NoKeyRotatedYet_ReturnsUnavailable()
    {
        // Arrange
        _store.GetLatestKeyPairAsync(Arg.Any<CancellationToken>()).Returns((SigningKeyPair?)null);

        // Act
        RpcException error = await Should.ThrowAsync<RpcException>(() => SignTokenAsync(UserId.ToString()));

        // Assert
        error.StatusCode.ShouldBe(StatusCode.Unavailable);
    }

    [Fact]
    public async Task SignToken_LatestKeyExpiresWithinATokenLifetime_ReturnsUnavailable()
    {
        // Arrange
        LatestKeyIs(KeyId, Now.Add(TokenLifetime));

        // Act
        RpcException error = await Should.ThrowAsync<RpcException>(() => SignTokenAsync(UserId.ToString()));

        // Assert
        error.StatusCode.ShouldBe(StatusCode.Unavailable);
    }

    [Fact]
    public async Task SignToken_DatabaseUnavailable_ReturnsUnavailable()
    {
        // Arrange
        _store.GetLatestKeyPairAsync(Arg.Any<CancellationToken>())
            .ThrowsAsync(new DatabaseUnavailableException(new TimeoutException()));

        // Act
        RpcException error = await Should.ThrowAsync<RpcException>(() => SignTokenAsync(UserId.ToString()));

        // Assert
        error.StatusCode.ShouldBe(StatusCode.Unavailable);
    }

    [Fact]
    public async Task SignToken_UnexpectedFailure_ReturnsInternalWithoutDetails()
    {
        // Arrange
        _store.GetLatestKeyPairAsync(Arg.Any<CancellationToken>())
            .ThrowsAsync(new InvalidOperationException("connection string leaked"));

        // Act
        RpcException error = await Should.ThrowAsync<RpcException>(() => SignTokenAsync(UserId.ToString()));

        // Assert
        error.StatusCode.ShouldBe(StatusCode.Internal);
        error.Status.Detail.ShouldNotContain("connection string");
    }

    [Fact]
    public async Task GetVerificationKeys_KeysInStore_ReturnsEachKeyWithItsExpiry()
    {
        // Arrange
        PublicSigningKey older = TestKeys.Create(KeyId, Now.AddHours(1)).PublicKey;
        PublicSigningKey newer = TestKeys.Create(NewerKeyId, Now.AddDays(1)).PublicKey;
        _store.GetVerificationKeysAsync(Arg.Any<CancellationToken>()).Returns([older, newer]);

        // Act
        GetVerificationKeysResponse response = await GetVerificationKeysAsync();

        // Assert
        response.Keys.Select(k => k.KeyId).ShouldBe([KeyId.ToString(), NewerKeyId.ToString()]);
        response.Keys.Select(k => k.PublicKey.ToByteArray()).ShouldBe([older.PublicKey, newer.PublicKey]);
        response.Keys.Select(k => k.ExpiresAt.ToDateTimeOffset()).ShouldBe([Now.AddHours(1), Now.AddDays(1)]);
    }
}
