using KeyProvider.Keys;
using KeyProvider.Worker;

using Weatherlab.EntityFrameworkCore.AppendOnly;

WebApplicationBuilder build = WebApplication.CreateBuilder(args);

build.Services.AddSingleton(TimeProvider.System);
build.Services.AddSingleton<AppendOnlyInterceptor>();

build.Services.AddSingleton<Ed25519KeyGenerator>();

build.Services.Configure<KeyRotationOptions>(build.Configuration.GetSection("KeyRotation"));
build.Services.AddHostedService<KeyRotationWorker>();

WebApplication app = build.Build();

await app.RunAsync();
