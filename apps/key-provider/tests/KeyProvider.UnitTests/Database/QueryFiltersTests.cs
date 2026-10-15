using KeyProvider.Database;
using KeyProvider.KeyRotation;
using KeyProvider.SigningKeys;

using Microsoft.Extensions.Time.Testing;

using Shouldly;

namespace KeyProvider.UnitTests.Database;

public sealed class QueryFiltersTests
{
    private static readonly DateTimeOffset Now = new(2026, 1, 1, 0, 0, 0, TimeSpan.Zero);

    private readonly FakeTimeProvider _clock = new(Now);

    private static KeyRotationJob Job(bool success, DateTimeOffset finishedAt)
    {
        return new KeyRotationJob
        {
            KeyPairRotationJobId = Guid.NewGuid(), StartedAt = finishedAt, FinishedAt = finishedAt, Success = success
        };
    }

    private static KeyPairRecord KeyExpiringAt(DateTimeOffset expiresAt)
    {
        KeyRotationJob job = Job(success: true, Now);
        return new KeyPairRecord
        {
            KeyPairId = Guid.NewGuid(),
            PublicKey = [],
            PrivateKey = [],
            ExpiresAt = expiresAt,
            KeyPairRotationJobId = job.KeyPairRotationJobId,
            RotationJob = job
        };
    }

    [Fact]
    public void Verifiable_KeyExpiringExactlyNow_IsLeftOut()
    {
        // Arrange
        KeyPairRecord[] keys = [KeyExpiringAt(Now)];

        // Act
        KeyPairRecord[] result = keys.AsQueryable().Verifiable(_clock).ToArray();

        // Assert
        result.ShouldBeEmpty();
    }

    [Fact]
    public void Verifiable_KeyExpiringOneTickAfterNow_IsKept()
    {
        // Arrange
        KeyPairRecord key = KeyExpiringAt(Now.AddTicks(1));

        // Act
        KeyPairRecord[] result = new[] { key }.AsQueryable().Verifiable(_clock).ToArray();

        // Assert
        result.ShouldBe([key]);
    }

    [Fact]
    public void LatestSuccessfulFirst_NewerFailedJob_ReturnsTheOlderSuccessfulJobFirst()
    {
        // Arrange
        KeyRotationJob older = Job(success: true, Now.AddHours(-2));
        KeyRotationJob newest = Job(success: true, Now.AddHours(-1));
        KeyRotationJob failed = Job(success: false, Now);

        // Act
        KeyRotationJob[] result = new[] { older, failed, newest }.AsQueryable().LatestSuccessfulFirst().ToArray();

        // Assert
        result.ShouldBe([newest, older]);
    }
}
