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
        Type clrType = entityTypeBuilder.Metadata.ClrType;
        if (clrType.IsDefined(typeof(AppendOnlyAttribute), true))
        {
            entityTypeBuilder.HasAnnotation(
                AppendOnlyAttribute.AnnotationName, true, true);
        }
    }

    public void ProcessModelFinalizing(IConventionModelBuilder modelBuilder,
        IConventionContext<IConventionModelBuilder> context)
    {
        List<string> errors = new();

        foreach (IConventionEntityType entityType in modelBuilder.Metadata.GetEntityTypes().ToList())
        {
            if (entityType.FindAnnotation(AppendOnlyAttribute.AnnotationName)?.Value is not true)
            {
                continue;
            }

            string name = entityType.DisplayName();

            foreach (IConventionForeignKey foreignKey in entityType.GetForeignKeys().ToList())
            {
                foreignKey.Builder.OnDelete(DeleteBehavior.Restrict, true);

                if (foreignKey.DeleteBehavior is not (DeleteBehavior.Restrict or DeleteBehavior.NoAction
                    or DeleteBehavior.ClientNoAction))
                {
                    errors.Add($"{name}: relationship to {foreignKey.PrincipalEntityType.DisplayName()} " +
                               $"uses {foreignKey.DeleteBehavior}; append-only tables require Restrict.");
                }
            }
        }

        if (errors.Count > 0)
        {
            throw new InvalidOperationException(
                "Append-only model errors:" + Environment.NewLine + string.Join(Environment.NewLine, errors));
        }
    }
}
