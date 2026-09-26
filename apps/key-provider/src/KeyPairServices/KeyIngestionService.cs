using KeyProvider.Data;
using KeyProvider.Models;

using Microsoft.Extensions.Options;

namespace KeyProvider.KeyPairServices;

internal sealed class KeyIngestionService(KeyProviderDbContext context, IOptions<KeyIngestionOptions> options)
{
    private readonly KeyIngestionOptions _options = options.Value;

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
