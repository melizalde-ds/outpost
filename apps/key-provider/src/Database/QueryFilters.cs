using KeyProvider.KeyRotation;
using KeyProvider.SigningKeys;

namespace KeyProvider.Database;

internal static class QueryFilters
{
    internal static IQueryable<KeyPairRecord> NewestFirst(this IQueryable<KeyPairRecord> query)
    {
        return query.OrderByDescending(p => p.CreatedAt);
    }

    internal static IQueryable<KeyPairRecord> JwkOrdering(this IQueryable<KeyPairRecord> query)
    {
        return query.OrderBy(p => p.CreatedAt);
    }

    internal static IQueryable<KeyPairRecord> Verifiable(this IQueryable<KeyPairRecord> query,
        TimeProvider timeProvider)
    {
        return query.Where(r => r.ExpiresAt > timeProvider.GetUtcNow());
    }

    internal static IQueryable<KeyRotationJob> LatestSuccessfulFirst(this IQueryable<KeyRotationJob> query)
    {
        return query.Where(k => k.Success == true).OrderByDescending(k => k.FinishedAt);
    }
}
