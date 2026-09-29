using System.Data.Common;

using Microsoft.EntityFrameworkCore;
using Microsoft.EntityFrameworkCore.ChangeTracking;
using Microsoft.EntityFrameworkCore.Diagnostics;

namespace Weatherlab.EntityFrameworkCore.AppendOnly;

public sealed class AppendOnlyInterceptor : SaveChangesInterceptor
{
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

    public override void SaveChangesFailed(DbContextErrorEventData eventData)
    {
        ArgumentNullException.ThrowIfNull(eventData);
        Translate(eventData.Exception);
    }

    public override Task SaveChangesFailedAsync(DbContextErrorEventData eventData,
        CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(eventData);
        Translate(eventData.Exception);
        return Task.CompletedTask;
    }

    private static void Translate(Exception exception)
    {
        for (Exception? current = exception; current is not null; current = current.InnerException)
        {
            if (current is DbException { SqlState: AppendOnlySqlState.Violation } dbException)
            {
                throw new AppendOnlyViolationException(dbException.Message, exception);
            }
        }
    }

    private static void Apply(DbContext? context)
    {
        if (context is null)
        {
            return;
        }

        List<string> violations = new();

        foreach (EntityEntry entry in context.ChangeTracker.Entries())
        {
            if (entry.Metadata.FindAnnotation(AppendOnlyAttribute.AnnotationName)?.Value is not true)
            {
                continue;
            }

            if (entry.State is EntityState.Modified or EntityState.Deleted)
            {
                violations.Add($"{entry.Metadata.DisplayName()} ({entry.State})");
            }
        }

        if (violations.Count > 0)
        {
            throw new AppendOnlyViolationException(
                "Append-only entities cannot be updated or deleted: " + string.Join(", ", violations));
        }
    }
}
