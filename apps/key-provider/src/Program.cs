using KeyProvider.CryptoKeys;
using KeyProvider.Worker;

using Weatherlab.EntityFrameworkCore.AppendOnly;
using Weatherlab.EntityFrameworkCore.CreatedAt;

WebApplicationBuilder build = WebApplication.CreateBuilder(args);

build.Services.AddSingleton(TimeProvider.System);
build.Services.AddSingleton<AppendOnlyInterceptor>();
build.Services.AddSingleton<CreatedAtInterceptor>();

build.Services.AddSingleton<Ed25519KeyGenerator>();

build.Services.Configure<KeyRotationOptions>(build.Configuration.GetSection("KeyRotation"));
build.Services.AddHostedService<KeyRotationWorker>();

WebApplication app = build.Build();

await app.RunAsync();
