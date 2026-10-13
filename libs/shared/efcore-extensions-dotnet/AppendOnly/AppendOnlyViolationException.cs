namespace Outpost.EntityFrameworkCore.AppendOnly;

public sealed class AppendOnlyViolationException : Exception
{
    public AppendOnlyViolationException()
    {
    }

    public AppendOnlyViolationException(string message) : base(message)
    {
    }

    public AppendOnlyViolationException(string message, Exception innerException) : base(message, innerException)
    {
    }
}
