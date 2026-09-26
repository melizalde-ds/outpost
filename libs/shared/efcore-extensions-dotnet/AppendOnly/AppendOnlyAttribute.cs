namespace Weatherlab.EntityFrameworkCore.AppendOnly;

[AttributeUsage(AttributeTargets.Class)]
public sealed class AppendOnlyAttribute : Attribute
{
    public const string AnnotationName = "AppendOnly";
    public const string CreatedAtAnnotationName = "AppendOnly:CreatedAt";
}
