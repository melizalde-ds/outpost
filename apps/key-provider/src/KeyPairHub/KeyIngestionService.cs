using KeyProvider.Data;
using KeyProvider.Models;

namespace KeyProvider.KeyPairHub;

internal sealed class KeyIngestionService(KeyProviderDbContext context)
{
    internal async Task SaveRotationAsync(KeyPairRecord record, CancellationToken ct = default)
    {
        if (!record.RotationJob.Success)
        {
            throw new ArgumentException("A key pair can only be saved with a successful rotation job.",
                nameof(record));
        }

        context.KeyPairRecords.Add(record);

        await context.SaveChangesAsync(ct);
    }

    internal async Task SaveFailedRotationAsync(KeyRotationJob job, CancellationToken ct = default)
    {
        if (job.Success)
        {
            throw new ArgumentException("Expected a failed rotation job.", nameof(job));
        }

        context.KeyRotationJobs.Add(job);

        await context.SaveChangesAsync(ct);
    }
}
