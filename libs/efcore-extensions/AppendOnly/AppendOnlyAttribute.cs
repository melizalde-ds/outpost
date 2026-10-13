namespace Outpost.EntityFrameworkCore.AppendOnly;

[AttributeUsage(AttributeTargets.Class)]
public sealed class AppendOnlyAttribute : Attribute
{
    public const string AnnotationName = "AppendOnly";
}
