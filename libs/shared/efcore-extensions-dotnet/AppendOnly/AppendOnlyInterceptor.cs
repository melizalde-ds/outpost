using Microsoft.EntityFrameworkCore;
using Microsoft.EntityFrameworkCore.ChangeTracking;
using Microsoft.EntityFrameworkCore.Diagnostics;

namespace Weatherlab.EntityFrameworkCore.AppendOnly;

public sealed class AppendOnlyInterceptor(TimeProvider timeProvider) : SaveChangesInterceptor
{
    private readonly TimeProvider _timeProvider = timeProvider;

    public override InterceptionResult<int> SavingChanges(DbContextEventData eventData,
        InterceptionResult<int> result)
    {
        ArgumentNullException.ThrowIfNull(eventData);
        Apply(eventData.Context);
        return result;
    }

    public override ValueTask<InterceptionResult<int>> SavingChangesAsync(DbContextEventData eventData,
        InterceptionResult<int> result, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(eventData);
        Apply(eventData.Context);
        return ValueTask.FromResult(result);
    }

    private void Apply(DbContext? context)
    {
        if (context is null)
        {
            return;
        }

        DateTimeOffset now = _timeProvider.GetUtcNow();
        var violations = new List<string>();

        foreach (EntityEntry entry in context.ChangeTracker.Entries())
        {
            if (entry.Metadata.FindAnnotation(AppendOnlyAttribute.AnnotationName)?.Value is not true)
            {
                continue;
            }

            switch (entry.State)
            {
                case EntityState.Added:
                    var createdAtProperty = (string)entry.Metadata
                        .FindAnnotation(AppendOnlyAttribute.CreatedAtAnnotationName)!.Value!;
                    entry.Property(createdAtProperty).CurrentValue = now;
                    break;

                case EntityState.Modified or EntityState.Deleted:
                    violations.Add($"{entry.Metadata.DisplayName()} ({entry.State})");
                    break;
            }
        }

        if (violations.Count > 0)
        {
            throw new InvalidOperationException(
                "Append-only entities cannot be updated or deleted: " + string.Join(", ", violations));
        }
    }
}
