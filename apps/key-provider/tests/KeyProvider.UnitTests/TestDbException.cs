using System.Data.Common;

namespace KeyProvider.UnitTests;

internal sealed class TestDbException(string message, bool isTransient, string? sqlState = null)
    : DbException(message)
{
    public override bool IsTransient => isTransient;
    public override string? SqlState => sqlState;
}
