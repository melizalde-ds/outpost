using KeyProvider.Database;

using Microsoft.EntityFrameworkCore;

namespace KeyProvider.KeyRotation;

internal sealed class RotationJobStore(KeyProviderDbContext context)
{
    internal async Task SaveFailedRotationAsync(KeyRotationJob job, CancellationToken ct = default)
    {
        if (job.Success)
        {
            throw new ArgumentException("Expected a failed rotation job.", nameof(job));
        }

        context.KeyRotationJobs.Add(job);

        await context.SaveChangesAsync(ct);
    }

    internal Task<KeyRotationJob?> GetRotationJobAsync(CancellationToken ct = default)
    {
        return context.KeyRotationJobs
            .AsNoTracking()
            .LatestSuccessfulFirst()
            .FirstOrDefaultAsync(ct);
    }
}
