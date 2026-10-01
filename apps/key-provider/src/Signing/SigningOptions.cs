using System.ComponentModel.DataAnnotations;

namespace KeyProvider.Signing;

public sealed record SigningOptions
{
    [Required] public string Algorithm { get; init; } = "EdDSA";

    [Required] public string Type { get; init; } = "JWT";

    [Required] public string Issuer { get; init; } = "WeatherLab";

    [Required] public string Audience { get; init; } = "WeatherLab";

    [Range(typeof(TimeSpan), "00:01:00", "00:15:00")]
    public TimeSpan TokenLifetime { get; init; } = TimeSpan.FromMinutes(5);
}
