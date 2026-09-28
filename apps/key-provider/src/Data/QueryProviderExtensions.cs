using KeyProvider.Models;

namespace KeyProvider.Data;

internal static class QueryProviderExtensions
{
    internal static IQueryable<KeyPairRecord> NewestFirst(this IQueryable<KeyPairRecord> query)
    {
        return query.OrderByDescending(p => p.CreatedAt);
    }

    internal static IQueryable<KeyPairRecord> Verifiable(this IQueryable<KeyPairRecord> query,
        TimeProvider timeProvider)
    {
        return query.Where(r => r.ExpiresAt > timeProvider.GetUtcNow());
    }

    internal static IQueryable<KeyRotationJob> NewestFirst(this IQueryable<KeyRotationJob> query)
    {
        return query.Where(k => k.Success == true).OrderByDescending(k => k.FinishedAt);
    }
}
