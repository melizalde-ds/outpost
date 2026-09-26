using System.Reflection;

using Microsoft.EntityFrameworkCore;
using Microsoft.EntityFrameworkCore.Metadata;
using Microsoft.EntityFrameworkCore.Metadata.Builders;
using Microsoft.EntityFrameworkCore.Metadata.Conventions;

namespace Weatherlab.EntityFrameworkCore.AppendOnly;

public sealed class AppendOnlyConvention : IEntityTypeAddedConvention, IModelFinalizingConvention
{
    public void ProcessEntityTypeAdded(IConventionEntityTypeBuilder entityTypeBuilder,
        IConventionContext<IConventionEntityTypeBuilder> context)
    {
        var clrType = entityTypeBuilder.Metadata.ClrType;
        if (clrType.IsDefined(typeof(AppendOnlyAttribute), inherit: true))
        {
            entityTypeBuilder.HasAnnotation(
                AppendOnlyAttribute.AnnotationName, true, fromDataAnnotation: true);
        }
    }

    public void ProcessModelFinalizing(IConventionModelBuilder modelBuilder,
        IConventionContext<IConventionModelBuilder> context)
    {
        var errors = new List<string>();

        foreach (IConventionEntityType entityType in modelBuilder.Metadata.GetEntityTypes().ToList())
        {
            if (entityType.FindAnnotation(AppendOnlyAttribute.AnnotationName)?.Value is not true)
            {
                continue;
            }

            var name = entityType.DisplayName();

            foreach (IConventionForeignKey foreignKey in entityType.GetForeignKeys().ToList())
            {
                foreignKey.Builder.OnDelete(DeleteBehavior.Restrict, fromDataAnnotation: true);

                if (foreignKey.DeleteBehavior is not (DeleteBehavior.Restrict or DeleteBehavior.NoAction
                    or DeleteBehavior.ClientNoAction))
                {
                    errors.Add($"{name}: relationship to {foreignKey.PrincipalEntityType.DisplayName()} " +
                               $"uses {foreignKey.DeleteBehavior}; append-only tables require Restrict.");
                }
            }

            var createdAt = entityType.GetProperties()
                .Where(p => p.PropertyInfo?.IsDefined(typeof(CreatedAtAttribute), inherit: true) == true)
                .ToList();

            if (createdAt.Count != 1 || createdAt[0].ClrType != typeof(DateTimeOffset))
            {
                errors.Add($"{name}: needs exactly one [CreatedAt] DateTimeOffset property.");
            }
            else
            {
                entityType.SetAnnotation(
                    AppendOnlyAttribute.CreatedAtAnnotationName, createdAt[0].Name, fromDataAnnotation: true);
            }
        }

        if (errors.Count > 0)
        {
            throw new InvalidOperationException(
                "Append-only model errors:" + Environment.NewLine + string.Join(Environment.NewLine, errors));
        }
    }
}
