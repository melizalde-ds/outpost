using KeyProvider.Data;
using KeyProvider.Models;

using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.Options;

namespace KeyProvider.KeyPairServices;

internal sealed class KeyRetrievalService(KeyProviderDbContext context, IOptions<KeyRetrievalOptions> options)
{
    private readonly KeyRetrievalOptions _options = options.Value;

    internal Task<KeyPairRecord?> GetKeysAsync(CancellationToken ct = default)
    {
        return context.KeyPairRecords.AsNoTracking().NewestFirst().FirstOrDefaultAsync(ct);
    }

    internal Task<KeyPairRecord[]> GetVerificationKeysAsync(CancellationToken ct = default)
    {
        return context.KeyPairRecords.AsNoTracking().CreatedSince(DateTimeOffset.UtcNow - _options.Since).NewestFirst()
            .ToArrayAsync(ct);
    }
}
