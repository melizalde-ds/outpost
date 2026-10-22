using System.Data.Common;

namespace KeyProvider.Database;

public sealed class DatabaseUnavailableException(Exception innerException)
    : Exception("The database is temporarily unavailable.", innerException);

internal static class DatabaseErrors
{
    internal static bool IsTemporary(Exception e)
    {
        for (Exception? current = e; current is not null; current = current.InnerException)
        {
            if (current is TimeoutException or DbException { IsTransient: true })
            {
                return true;
            }
        }

        return false;
    }
}
