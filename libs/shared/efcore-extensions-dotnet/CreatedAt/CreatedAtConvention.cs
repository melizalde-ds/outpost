using Microsoft.EntityFrameworkCore.Metadata;
using Microsoft.EntityFrameworkCore.Metadata.Builders;
using Microsoft.EntityFrameworkCore.Metadata.Conventions;

namespace Weatherlab.EntityFrameworkCore.CreatedAt;

public sealed class CreatedAtConvention : IModelFinalizingConvention
{
    public void ProcessModelFinalizing(IConventionModelBuilder modelBuilder,
        IConventionContext<IConventionModelBuilder> context)
    {
        List<string> errors = new();

        foreach (IConventionEntityType entityType in modelBuilder.Metadata.GetEntityTypes().ToList())
        {
            List<IConventionProperty> createdAt = entityType.GetProperties()
                .Where(p => p.PropertyInfo?.IsDefined(typeof(CreatedAtAttribute), true) == true)
                .ToList();

            if (createdAt.Count == 0)
            {
                continue;
            }

            string name = entityType.DisplayName();

            if (createdAt.Count > 1)
            {
                errors.Add($"{name}: has more than one [CreatedAt] property.");
            }
            else if (createdAt[0].ClrType != typeof(DateTimeOffset))
            {
                errors.Add($"{name}: [CreatedAt] property {createdAt[0].Name} must be DateTimeOffset.");
            }
            else
            {
                entityType.SetAnnotation(CreatedAtAttribute.AnnotationName, createdAt[0].Name,
                    true);
            }
        }

        if (errors.Count > 0)
        {
            throw new InvalidOperationException(
                "CreatedAt model errors:" + Environment.NewLine + string.Join(Environment.NewLine, errors));
        }
    }
}
