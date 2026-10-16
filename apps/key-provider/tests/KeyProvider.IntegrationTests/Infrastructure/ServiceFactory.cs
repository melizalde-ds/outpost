using Grpc.Net.Client;
using KeyProvider.KeyRotation;
using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.Mvc.Testing;
using Microsoft.AspNetCore.TestHost;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Time.Testing;
using Outpost.Key.V1;

namespace KeyProvider.IntegrationTests.Infrastructure;

/// <summary>The real key provider, in memory, on its own databases and a fake clock.</summary>
internal sealed class ServiceFactory(TestDatabases databases, bool keepRotationWorker = false)
    : WebApplicationFactory<Program>
{
    public static readonly DateTimeOffset Start = new(2026, 1, 1, 0, 0, 0, TimeSpan.Zero);

    public FakeTimeProvider Time { get; } = new(Start);

    protected override void ConfigureWebHost(IWebHostBuilder builder)
    {
        // Not Development, so only appsettings.json applies and user secrets aren't loaded.
        builder.UseEnvironment("IntegrationTest");
        builder.UseSetting("ConnectionStrings:KeyProvider", databases.KeyProvider);
        builder.UseSetting("ConnectionStrings:DataProtection", databases.DataProtection);
        builder.ConfigureTestServices(services =>
        {
            services.AddSingleton<TimeProvider>(Time);
            if (!keepRotationWorker)
            {
                services.Remove(
                    services.Single(s => s.ImplementationType == typeof(RotationWorker))
                );
            }
        });
    }

    public KeyService.KeyServiceClient CreateKeyClient()
    {
        GrpcChannel channel = GrpcChannel.ForAddress(
            Server.BaseAddress,
            new GrpcChannelOptions { HttpHandler = Server.CreateHandler() }
        );
        return new KeyService.KeyServiceClient(channel);
    }

    public Task<KeyRotationResult> RotateAsync(CancellationToken ct)
    {
        return Services.GetRequiredService<IKeyRotationService>().RotateAsync(ct);
    }
}
