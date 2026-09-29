using Microsoft.AspNetCore.DataProtection;
using Microsoft.EntityFrameworkCore;

using Weatherlab.EntityFrameworkCore.AppendOnly;
using Weatherlab.EntityFrameworkCore.CreatedAt;

namespace KeyProvider.Database;

internal static class DatabaseSetup
{
    internal static IServiceCollection AddKeyProviderDbContext(this IServiceCollection services,
        IConfiguration configuration)
    {
        services.AddSingleton<AppendOnlyInterceptor>();
        services.AddSingleton<CreatedAtInterceptor>();

        services.AddDbContextFactory<KeyProviderDbContext>((sp, options) => options
            .UseNpgsql(configuration.GetConnectionString("KeyProvider"))
            .UseSnakeCaseNamingConvention()
            .AddInterceptors(
                sp.GetRequiredService<AppendOnlyInterceptor>(),
                sp.GetRequiredService<CreatedAtInterceptor>()));

        return services;
    }

    internal static IServiceCollection AddDataProtectionDbContext(this IServiceCollection services,
        IConfiguration configuration)
    {
        services.AddDbContext<DataProtectionDbContext>(options =>
        {
            options.UseNpgsql(configuration.GetConnectionString("DataProtection"));
        });

        services.AddDataProtection().SetApplicationName("key-provider")
            .PersistKeysToDbContext<DataProtectionDbContext>();
        return services;
    }
}
