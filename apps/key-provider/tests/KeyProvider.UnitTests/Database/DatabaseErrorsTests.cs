using KeyProvider.Database;

using Microsoft.EntityFrameworkCore;

using Shouldly;

namespace KeyProvider.UnitTests.Database;

public sealed class DatabaseErrorsTests
{
    public static TheoryData<Exception> TemporaryErrors => new()
    {
        new TimeoutException(),
        new TestDbException("connection reset", isTransient: true),
        new InvalidOperationException("save failed",
            new DbUpdateException("save failed", new TestDbException("connection reset", isTransient: true)))
    };

    public static TheoryData<Exception> LastingErrors => new()
    {
        new TestDbException("rejected", isTransient: false),
        new InvalidOperationException("save failed", new TestDbException("rejected", isTransient: false))
    };

    [Theory]
    [MemberData(nameof(TemporaryErrors))]
    public void IsTemporary_TemporaryErrorAtAnyDepth_ReturnsTrue(Exception error)
    {
        // Act
        bool temporary = DatabaseErrors.IsTemporary(error);

        // Assert
        temporary.ShouldBeTrue();
    }

    [Theory]
    [MemberData(nameof(LastingErrors))]
    public void IsTemporary_LastingError_ReturnsFalse(Exception error)
    {
        // Act
        bool temporary = DatabaseErrors.IsTemporary(error);

        // Assert
        temporary.ShouldBeFalse();
    }
}
