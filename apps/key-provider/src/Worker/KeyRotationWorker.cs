using KeyProvider.CryptoKeys;
using KeyProvider.KeyPairServices;
using KeyProvider.Models;

using Microsoft.Extensions.Options;

using Org.BouncyCastle.Crypto.Parameters;

namespace KeyProvider.Worker;

internal sealed class KeyRotationWorker(
    IOptions<KeyRotationOptions> options,
    ILogger<KeyRotationWorker> logger,
    Ed25519KeyGenerator engine,
    IServiceScopeFactory scopeFactory,
    TimeProvider timeProvider) : BackgroundService
{
    private const int MaxErrorMessageLength = 512;

    private readonly KeyRotationOptions _options = options.Value;

    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
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
                PrivateKey = keyPair.PrivateKey.GetEncoded(),
                ExpiresAt = startTime + _options.Timeout,
                KeyPairRotationJobId = job.KeyPairRotationJobId,
                RotationJob = job
            };

            await ingestion.SaveRotationAsync(record, stoppingToken);
        }
        catch (OperationCanceledException e) when (stoppingToken.IsCancellationRequested)
        {
            logger.LogInformation(e, "Cancellation requested");
            // stoppingToken is already cancelled, so the failure write can't use it.
            await TrySaveFailedRotationAsync(jobId, startTime, e, CancellationToken.None);
            throw;
        }
        catch (Exception e)
        {
            logger.LogError(e, "Unexpected error");
            await TrySaveFailedRotationAsync(jobId, startTime, e, stoppingToken);
        }
    }

    private async Task TrySaveFailedRotationAsync(Guid jobId, DateTimeOffset startTime, Exception error,
        CancellationToken ct)
    {
        try
        {
            await using AsyncServiceScope scope = scopeFactory.CreateAsyncScope();
            KeyIngestionService ingestion = scope.ServiceProvider.GetRequiredService<KeyIngestionService>();

            string errorMessage = $"{error.GetType().Name}: {error.Message}";
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
}
