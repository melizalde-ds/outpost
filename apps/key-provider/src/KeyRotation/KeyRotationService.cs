using System.Data.Common;

using KeyProvider.Database;
using KeyProvider.KeyGeneration;
using KeyProvider.SigningKeys;

using Microsoft.AspNetCore.DataProtection;
using Microsoft.Extensions.Options;

using Org.BouncyCastle.Crypto.Parameters;

using Outpost.EntityFrameworkCore.AppendOnly;

namespace KeyProvider.KeyRotation;

internal sealed class KeyRotationService(
    ILogger<KeyRotationService> logger,
    IOptions<KeyRotationOptions> options,
    Ed25519KeyGenerator keyGenerator,
    IServiceScopeFactory scopeFactory,
    TimeProvider timeProvider,
    IDataProtectionProvider dataProtectionProvider) : IKeyRotationService
{
    private const int MaxErrorMessageLength = 512;
    private readonly KeyRotationOptions _options = options.Value;

    private readonly IDataProtector _protector =
        dataProtectionProvider.CreateProtector(KeyPairRecord.PrivateKeyProtectionPurpose);

    public async Task<KeyRotationResult> RotateAsync(CancellationToken ct = default)
    {
        DateTimeOffset startTime = timeProvider.GetUtcNow();
        Guid jobId = Guid.CreateVersion7(startTime);
        try
        {
            logger.LogInformation("Job with ID {JobId} has started at {TimeNow}", jobId, startTime);
            AsymmetricKeyPair<Ed25519PrivateKeyParameters, Ed25519PublicKeyParameters> keyPair =
                keyGenerator.GenerateKeyPair();
            logger.LogInformation("[{JobId}] - Keypair generated", jobId);

            await using AsyncServiceScope scope = scopeFactory.CreateAsyncScope();
            IKeyPairStore keyPairStore = scope.ServiceProvider.GetRequiredService<IKeyPairStore>();

            KeyRotationJob job = new() { KeyPairRotationJobId = jobId, StartedAt = startTime, Success = true };

            KeyPairRecord record = new()
            {
                KeyPairId = Guid.CreateVersion7(startTime),
                PublicKey = keyPair.PublicKey.GetEncoded(),
                PrivateKey = _protector.Protect(keyPair.PrivateKey.GetEncoded()),
                ExpiresAt = startTime + _options.Lifespan,
                KeyPairRotationJobId = job.KeyPairRotationJobId,
                RotationJob = job
            };

            await keyPairStore.SaveKeyPairAsync(record, ct);
            return new KeyRotationSucceeded(startTime, timeProvider.GetUtcNow(), record.KeyPairId);
        }
        catch (OperationCanceledException e) when (ct.IsCancellationRequested)
        {
            logger.LogInformation(e, "Cancellation requested");
            await TrySaveFailedRotationAsync(jobId, startTime, e, CancellationToken.None);
            throw;
        }
        catch (Exception e)
        {
            logger.LogError(e, "Unexpected error");
            await TrySaveFailedRotationAsync(jobId, startTime, e, ct);
            return new KeyRotationFailed(startTime, timeProvider.GetUtcNow(), Describe(e), IsTransient(e), e);
        }
    }

    private static bool IsTransient(Exception e)
    {
        if (e is AppendOnlyViolationException ||
            FindDbException(e, DatabaseErrorCodes.RotationJobNotSuccessful) is not null)
        {
            return false;
        }

        return e is TimeoutException or DbException { IsTransient: true }
               || e.InnerException is TimeoutException or DbException { IsTransient: true };
    }

    private static string Describe(Exception e)
    {
        Exception cause = FindDbException(e, DatabaseErrorCodes.RotationJobNotSuccessful) ?? e;
        return $"{cause.GetType().Name}: {cause.Message}";
    }

    private static DbException? FindDbException(Exception e, string sqlState)
    {
        for (Exception? current = e; current is not null; current = current.InnerException)
        {
            if (current is DbException dbException && dbException.SqlState == sqlState)
            {
                return dbException;
            }
        }

        return null;
    }

    private async Task TrySaveFailedRotationAsync(Guid jobId, DateTimeOffset startTime, Exception error,
        CancellationToken ct)
    {
        try
        {
            await using AsyncServiceScope scope = scopeFactory.CreateAsyncScope();
            IRotationJobStore jobStore = scope.ServiceProvider.GetRequiredService<IRotationJobStore>();

            string errorMessage = Describe(error);
            KeyRotationJob job = new()
            {
                KeyPairRotationJobId = jobId,
                StartedAt = startTime,
                Success = false,
                ErrorMessage = errorMessage[..Math.Min(errorMessage.Length, MaxErrorMessageLength)]
            };

            await jobStore.SaveFailedRotationAsync(job, ct);
        }
        catch (Exception e)
        {
            logger.LogError(e, "[{JobId}] - Failed to record failed rotation", jobId);
        }
    }

    public async Task<KeyRotationJob?> GetLatestSuccessfulJobAsync(CancellationToken ct)
    {
        await using AsyncServiceScope scope = scopeFactory.CreateAsyncScope();
        IRotationJobStore jobStore = scope.ServiceProvider.GetRequiredService<IRotationJobStore>();

        return await jobStore.GetLatestSuccessfulJobAsync(ct);
    }
}
