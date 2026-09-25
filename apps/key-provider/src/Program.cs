using KeyProvider.Keys;
using KeyProvider.Worker;


var build = WebApplication.CreateBuilder(args);

build.Services.AddSingleton<Ed25519KeyGenerator>();

build.Services.Configure<KeyRotationOptions>(build.Configuration.GetSection("KeyRotation"));
build.Services.AddHostedService<KeyRotationWorker>();

var app = build.Build();

await app.RunAsync();
