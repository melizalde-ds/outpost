using KeyProvider.Data;
using KeyProvider.Models;

using Microsoft.EntityFrameworkCore;

namespace KeyProvider.KeyPairHub;

internal sealed class KeyRetrievalService(
    KeyProviderDbContext context,
    TimeProvider timeProvider)
{
    internal Task<KeyPairRecord?> GetKeysAsync(CancellationToken ct = default)
    {
        return context.KeyPairRecords.AsNoTracking().NewestFirst().FirstOrDefaultAsync(ct);
    }

    internal Task<KeyPairRecord[]> GetVerificationKeysAsync(CancellationToken ct = default)
    {
        return context.KeyPairRecords.AsNoTracking().Verifiable(timeProvider).NewestFirst()
            .ToArrayAsync(ct);
    }

    internal Task<KeyRotationJob?> GetRotationJobAsync(CancellationToken ct = default)
    {
        return context.KeyRotationJobs.AsNoTracking().NewestFirst().FirstOrDefaultAsync(ct);
    }
}
