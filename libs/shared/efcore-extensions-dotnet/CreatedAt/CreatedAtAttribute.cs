namespace Weatherlab.EntityFrameworkCore.CreatedAt;

[AttributeUsage(AttributeTargets.Property)]
public sealed class CreatedAtAttribute : Attribute
{
    public const string AnnotationName = "CreatedAt";
}
