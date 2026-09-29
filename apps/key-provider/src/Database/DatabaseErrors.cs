namespace KeyProvider.Database;

public sealed class DatabaseUnavailableException(Exception innerException)
    : Exception("The database is temporarily unavailable.", innerException);
