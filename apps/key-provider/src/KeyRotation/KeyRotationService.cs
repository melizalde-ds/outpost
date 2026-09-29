using System.Data.Common;

using KeyProvider.CryptoKeys;
using KeyProvider.Data;
using KeyProvider.KeyPairHub;
using KeyProvider.Models;

using Microsoft.AspNetCore.DataProtection;
using Microsoft.Extensions.Options;

using Org.BouncyCastle.Crypto.Parameters;

using Weatherlab.EntityFrameworkCore.AppendOnly;

namespace KeyProvider.KeyRotation;

internal sealed class KeyRotationService(
    ILogger<KeyRotationService> logger,
    IOptions<KeyRotationOptions> options,
    Ed25519KeyGenerator engine,
    IServiceScopeFactory scopeFactory,
    TimeProvider timeProvider,
    IDataProtectionProvider dataProtectionProvider)
{
    private const int MaxErrorMessageLength = 512;
    private readonly KeyRotationOptions _options = options.Value;

    private readonly IDataProtector _protector =
        dataProtectionProvider.CreateProtector(KeyPairRecord.PrivateKeyProtectionPurpose);

    internal async Task<KeyRotationResult> KeyRotationAsync(CancellationToken cts = default)
    {
        DateTimeOffset startTime = timeProvider.GetUtcNow();
        Guid jobId = Guid.CreateVersion7(startTime);
        try
        {
            logger.LogInformation("Job with ID {JobID} has started at {TimeNow}", jobId, startTime);
            AsymmetricKeyPair<Ed25519PrivateKeyParameters, Ed25519PublicKeyParameters> keyPair =
                engine.GenerateKeyPair();
            logger.LogInformation("[{JobID}] - Keypair generated", jobId);

            await using AsyncServiceScope scope = scopeFactory.CreateAsyncScope();
            KeyIngestionService ingestion = scope.ServiceProvider.GetRequiredService<KeyIngestionService>();

            KeyRotationJob job = new()
            {
                KeyPairRotationJobId = jobId,
                StartedAt = startTime,
                Success = true,
                FinishedAt = timeProvider.GetUtcNow()
            };

            KeyPairRecord record = new()
            {
                KeyPairId = Guid.CreateVersion7(startTime),
                CreatedAt = startTime,
                PublicKey = keyPair.PublicKey.GetEncoded(),
                PrivateKey = _protector.Protect(keyPair.PrivateKey.GetEncoded()),
                ExpiresAt = startTime + _options.Lifespan,
                KeyPairRotationJobId = job.KeyPairRotationJobId,
                RotationJob = job
            };

            await ingestion.SaveRotationAsync(record, cts);
            return new KeyRotationSucceeded(startTime, timeProvider.GetUtcNow(), record.KeyPairId);
        }
        catch (OperationCanceledException e) when (cts.IsCancellationRequested)
        {
            logger.LogInformation(e, "Cancellation requested");
            await TrySaveFailedRotationAsync(jobId, startTime, e, CancellationToken.None);
            throw;
        }
        catch (Exception e)
        {
            logger.LogError(e, "Unexpected error");
            await TrySaveFailedRotationAsync(jobId, startTime, e, cts);
            return new KeyRotationFailed(startTime, timeProvider.GetUtcNow(), Describe(e), IsTransient(e), e);
        }
    }

    private static bool IsTransient(Exception e)
    {
        if (e is AppendOnlyViolationException ||
            FindDbException(e, KeyProviderSqlState.RotationJobNotSuccessful) is not null)
        {
            return false;
        }

        return e is TimeoutException or DbException { IsTransient: true }
               || e.InnerException is TimeoutException or DbException { IsTransient: true };
    }

    private static string Describe(Exception e)
    {
        Exception cause = FindDbException(e, KeyProviderSqlState.RotationJobNotSuccessful) ?? e;
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
            KeyIngestionService ingestion = scope.ServiceProvider.GetRequiredService<KeyIngestionService>();

            string errorMessage = Describe(error);
            KeyRotationJob job = new()
            {
                KeyPairRotationJobId = jobId,
                StartedAt = startTime,
                Success = false,
                FinishedAt = timeProvider.GetUtcNow(),
                ErrorMessage = errorMessage.Length > MaxErrorMessageLength
                    ? errorMessage[..MaxErrorMessageLength]
                    : errorMessage
            };

            await ingestion.SaveFailedRotationAsync(job, ct);
        }
        catch (Exception e)
        {
            logger.LogError(e, "[{JobID}] - Failed to record failed rotation", jobId);
        }
    }

    internal async Task<KeyRotationJob?> GetLatestRotationAsync(CancellationToken ct)
    {
        await using AsyncServiceScope scope = scopeFactory.CreateAsyncScope();
        KeyRetrievalService retrieval = scope.ServiceProvider.GetRequiredService<KeyRetrievalService>();

        return await retrieval.GetRotationJobAsync(ct);
    }
}
