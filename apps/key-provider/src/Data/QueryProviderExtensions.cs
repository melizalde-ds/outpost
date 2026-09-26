using KeyProvider.Models;

namespace KeyProvider.Data;

internal static class QueryProviderExtensions
{
    internal static IQueryable<KeyPairRecord> NewestFirst(this IQueryable<KeyPairRecord> query)
    {
        return query.OrderByDescending(p => p.CreatedAt);
    }

    internal static IQueryable<KeyPairRecord> CreatedSince(this IQueryable<KeyPairRecord> query, DateTimeOffset since)
    {
        return query.OrderBy(p => p.CreatedAt).Where(k => k.CreatedAt > since);
    }
}
