using KeyProvider.Database;

using Microsoft.EntityFrameworkCore;

namespace KeyProvider.KeyRotation;

internal sealed class RotationJobStore(KeyProviderDbContext context) : IRotationJobStore
{
    public async Task SaveFailedRotationAsync(KeyRotationJob job, CancellationToken ct = default)
    {
        if (job.Success)
        {
            throw new ArgumentException("Expected a failed rotation job.", nameof(job));
        }

        context.KeyRotationJobs.Add(job);

        await context.SaveChangesAsync(ct);
    }

    public Task<KeyRotationJob?> GetLatestSuccessfulJobAsync(CancellationToken ct = default)
    {
        return context.KeyRotationJobs
            .AsNoTracking()
            .LatestSuccessfulFirst()
            .FirstOrDefaultAsync(ct);
    }
}
