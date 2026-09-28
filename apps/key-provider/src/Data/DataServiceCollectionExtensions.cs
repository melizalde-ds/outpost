using Microsoft.EntityFrameworkCore;

using Weatherlab.EntityFrameworkCore.AppendOnly;
using Weatherlab.EntityFrameworkCore.CreatedAt;

namespace KeyProvider.Data;

internal static class DataServiceCollectionExtensions
{
    internal static IServiceCollection AddKeyProviderDbContext(this IServiceCollection services,
        IConfiguration configuration)
    {
        services.AddSingleton<AppendOnlyInterceptor>();
        services.AddSingleton<CreatedAtInterceptor>();

        services.AddDbContextFactory<KeyProviderDbContext>((sp, options) => options
            .UseNpgsql(configuration.GetConnectionString("KeyProvider"))
            .AddInterceptors(
                sp.GetRequiredService<AppendOnlyInterceptor>(),
                sp.GetRequiredService<CreatedAtInterceptor>()));

        return services;
    }
}
