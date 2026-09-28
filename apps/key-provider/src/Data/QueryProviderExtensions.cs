using KeyProvider.Models;

namespace KeyProvider.Data;

internal static class QueryProviderExtensions
{
    internal static IQueryable<KeyPairRecord> NewestFirst(this IQueryable<KeyPairRecord> query)
    {
        return query.OrderByDescending(p => p.CreatedAt);
    }

    internal static IQueryable<KeyPairRecord> Verifiable(this IQueryable<KeyPairRecord> query,
        TimeProvider timeProvider, TimeSpan since)
    {
        DateTimeOffset target = timeProvider.GetUtcNow() - since;
        return query.Where(r => r.ExpiresAt > target);
    }
}
