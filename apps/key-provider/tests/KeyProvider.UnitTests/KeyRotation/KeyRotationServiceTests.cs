using KeyProvider.Database;
using KeyProvider.KeyGeneration;
using KeyProvider.KeyRotation;
using KeyProvider.SigningKeys;

using Microsoft.AspNetCore.DataProtection;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging.Abstractions;
using Microsoft.Extensions.Options;
using Microsoft.Extensions.Time.Testing;

using NSubstitute;
using NSubstitute.ExceptionExtensions;

using Org.BouncyCastle.Crypto.Parameters;

using Outpost.EntityFrameworkCore.AppendOnly;

using Shouldly;

namespace KeyProvider.UnitTests.KeyRotation;

public sealed class KeyRotationServiceTests : IDisposable
{
    private static readonly DateTimeOffset Now = new(2026, 1, 1, 0, 0, 0, TimeSpan.Zero);
    private static readonly TimeSpan Lifespan = TimeSpan.FromDays(1);

    private readonly IKeyPairStore _keys = Substitute.For<IKeyPairStore>();
    private readonly IRotationJobStore _jobs = Substitute.For<IRotationJobStore>();
    private readonly EphemeralDataProtectionProvider _dataProtection = new();
    private readonly ServiceProvider _services;
    private readonly KeyRotationService _sut;
    private KeyPairRecord? _savedKey;
    private KeyRotationJob? _failedRotation;

    public KeyRotationServiceTests()
    {
        _keys.SaveKeyPairAsync(Arg.Do<KeyPairRecord>(record => _savedKey = record), Arg.Any<CancellationToken>());
        _jobs.SaveFailedRotationAsync(Arg.Do<KeyRotationJob>(job => _failedRotation = job),
            Arg.Any<CancellationToken>());

        _services = new ServiceCollection()
            .AddScoped(_ => _keys)
            .AddScoped(_ => _jobs)
            .BuildServiceProvider();

        _sut = new KeyRotationService(
            NullLogger<KeyRotationService>.Instance,
            Options.Create(new KeyRotationOptions { Lifespan = Lifespan }),
            new Ed25519KeyGenerator(NullLogger<Ed25519KeyGenerator>.Instance),
            _services.GetRequiredService<IServiceScopeFactory>(),
            new FakeTimeProvider(Now),
            _dataProtection);
    }

    public void Dispose()
    {
        _services.Dispose();
    }

    private void SavingKeysFailsWith(Exception error)
    {
        _keys.SaveKeyPairAsync(Arg.Any<KeyPairRecord>(), Arg.Any<CancellationToken>()).ThrowsAsync(error);
    }

    [Fact]
    public async Task GetLatestSuccessfulJobAsync_JobInStore_ReturnsIt()
    {
        // Arrange
        KeyRotationJob job = new()
        {
            KeyPairRotationJobId = Guid.CreateVersion7(Now), StartedAt = Now, FinishedAt = Now, Success = true
        };
        _jobs.GetLatestSuccessfulJobAsync(Arg.Any<CancellationToken>()).Returns(job);

        // Act
        KeyRotationJob? latest = await _sut.GetLatestSuccessfulJobAsync(TestContext.Current.CancellationToken);

        // Assert
        latest.ShouldBeSameAs(job);
    }

    [Fact]
    public async Task RotateAsync_StoreAcceptsTheKey_ReturnsSuccessWithTheSavedKeyId()
    {
        // Act
        KeyRotationResult result = await _sut.RotateAsync(TestContext.Current.CancellationToken);

        // Assert
        result.ShouldBeOfType<KeyRotationSucceeded>().NewKeyId.ShouldBe(_savedKey.ShouldNotBeNull().KeyPairId);
    }

    [Fact]
    public async Task RotateAsync_StoreAcceptsTheKey_SavedKeyExpiresAfterTheLifespan()
    {
        // Act
        await _sut.RotateAsync(TestContext.Current.CancellationToken);

        // Assert
        _savedKey.ShouldNotBeNull().ExpiresAt.ShouldBe(Now + Lifespan);
    }

    [Fact]
    public async Task RotateAsync_StoreAcceptsTheKey_PrivateKeyIsSavedEncryptedAndMatchesThePublicKey()
    {
        // Arrange
        IDataProtector protector = _dataProtection.CreateProtector(KeyPairRecord.PrivateKeyProtectionPurpose);

        // Act
        await _sut.RotateAsync(TestContext.Current.CancellationToken);

        // Assert
        KeyPairRecord saved = _savedKey.ShouldNotBeNull();
        byte[] privateKey = protector.Unprotect(saved.PrivateKey);
        saved.PrivateKey.ShouldNotBe(privateKey);
        new Ed25519PrivateKeyParameters(privateKey).GeneratePublicKey().GetEncoded().ShouldBe(saved.PublicKey);
    }

    [Fact]
    public async Task RotateAsync_StoreAcceptsTheKey_SavesASuccessfulJobLinkedToTheKey()
    {
        // Act
        await _sut.RotateAsync(TestContext.Current.CancellationToken);

        // Assert
        KeyPairRecord saved = _savedKey.ShouldNotBeNull();
        saved.RotationJob.Success.ShouldBeTrue();
        saved.RotationJob.StartedAt.ShouldBe(Now);
        saved.KeyPairRotationJobId.ShouldBe(saved.RotationJob.KeyPairRotationJobId);
    }

    [Fact]
    public async Task RotateAsync_TransientDatabaseErrorInsideDbUpdateException_ReturnsAFailureWorthRetrying()
    {
        // Arrange
        SavingKeysFailsWith(new DbUpdateException("save failed",
            new TestDbException("connection reset", isTransient: true)));

        // Act
        KeyRotationResult result = await _sut.RotateAsync(TestContext.Current.CancellationToken);

        // Assert
        result.ShouldBeOfType<KeyRotationFailed>().IsTransient.ShouldBeTrue();
    }

    [Fact]
    public async Task RotateAsync_TransientDatabaseErrorTwoLevelsDeep_ReturnsAFailureWorthRetrying()
    {
        // Arrange
        SavingKeysFailsWith(new InvalidOperationException("save failed",
            new DbUpdateException("save failed", new TestDbException("connection reset", isTransient: true))));

        // Act
        KeyRotationResult result = await _sut.RotateAsync(TestContext.Current.CancellationToken);

        // Assert
        result.ShouldBeOfType<KeyRotationFailed>().IsTransient.ShouldBeTrue();
    }

    [Fact]
    public async Task RotateAsync_AppendOnlyViolationCausedByTransientError_ReturnsAFailureNotWorthRetrying()
    {
        // Arrange
        SavingKeysFailsWith(new AppendOnlyViolationException("append-only",
            new TestDbException("connection reset", isTransient: true)));

        // Act
        KeyRotationResult result = await _sut.RotateAsync(TestContext.Current.CancellationToken);

        // Assert
        result.ShouldBeOfType<KeyRotationFailed>().IsTransient.ShouldBeFalse();
    }

    [Fact]
    public async Task RotateAsync_TransientDatabaseError_ReturnsAFailureWorthRetrying()
    {
        // Arrange
        SavingKeysFailsWith(new TestDbException("connection reset", isTransient: true));

        // Act
        KeyRotationResult result = await _sut.RotateAsync(TestContext.Current.CancellationToken);

        // Assert
        result.ShouldBeOfType<KeyRotationFailed>().IsTransient.ShouldBeTrue();
    }

    [Fact]
    public async Task RotateAsync_DatabaseRejectsTheRotation_ReturnsAFailureNotWorthRetrying()
    {
        // Arrange
        SavingKeysFailsWith(new TestDbException("rejected", isTransient: true,
            DatabaseErrorCodes.RotationJobNotSuccessful));

        // Act
        KeyRotationResult result = await _sut.RotateAsync(TestContext.Current.CancellationToken);

        // Assert
        result.ShouldBeOfType<KeyRotationFailed>().IsTransient.ShouldBeFalse();
    }

    [Fact]
    public async Task RotateAsync_DatabaseError_RecordsTheFailedRotationWithTheReason()
    {
        // Arrange
        SavingKeysFailsWith(new TestDbException("connection reset", isTransient: true));

        // Act
        await _sut.RotateAsync(TestContext.Current.CancellationToken);

        // Assert
        KeyRotationJob recorded = _failedRotation.ShouldNotBeNull();
        recorded.Success.ShouldBeFalse();
        recorded.ErrorMessage.ShouldNotBeNull().ShouldContain("connection reset");
    }

    [Fact]
    public async Task RotateAsync_DatabaseRejectsTheRotationInsideDbUpdateException_RecordsTheDatabaseReason()
    {
        // Arrange
        SavingKeysFailsWith(new DbUpdateException("save failed",
            new TestDbException("rejected", isTransient: false, DatabaseErrorCodes.RotationJobNotSuccessful)));

        // Act
        await _sut.RotateAsync(TestContext.Current.CancellationToken);

        // Assert
        _failedRotation.ShouldNotBeNull().ErrorMessage.ShouldBe("TestDbException: rejected");
    }

    [Fact]
    public async Task RotateAsync_VeryLongError_RecordsAMessageThatFitsTheColumn()
    {
        // Arrange
        SavingKeysFailsWith(new TestDbException(new string('x', 1_000), isTransient: false));

        // Act
        await _sut.RotateAsync(TestContext.Current.CancellationToken);

        // Assert
        _failedRotation.ShouldNotBeNull().ErrorMessage.ShouldNotBeNull().Length.ShouldBe(512);
    }

    [Fact]
    public async Task RotateAsync_Cancelled_StillRecordsTheFailedRotation()
    {
        // Arrange
        using CancellationTokenSource cancellation = new();
        await cancellation.CancelAsync();
        SavingKeysFailsWith(new OperationCanceledException(cancellation.Token));

        // Act
        await Should.ThrowAsync<OperationCanceledException>(() => _sut.RotateAsync(cancellation.Token));

        // Assert
        _failedRotation.ShouldNotBeNull().Success.ShouldBeFalse();
    }
}
