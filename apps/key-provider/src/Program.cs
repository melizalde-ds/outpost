using KeyProvider.CryptoKeys;
using KeyProvider.Data;
using KeyProvider.KeyPairServices;
using KeyProvider.Worker;

WebApplicationBuilder build = WebApplication.CreateBuilder(args);

build.Services.AddSingleton(TimeProvider.System);

build.Services.AddKeyProviderDbContext(build.Configuration);

build.Services.AddSingleton<Ed25519KeyGenerator>();

build.Services.AddOptionsWithValidateOnStart<KeyIngestionOptions>().BindConfiguration("KeyIngestion")
    .ValidateDataAnnotations();
build.Services.AddScoped<KeyIngestionService>();

build.Services.AddOptionsWithValidateOnStart<KeyRetrievalOptions>().BindConfiguration("KeyRetrieval")
    .ValidateDataAnnotations();
build.Services.AddScoped<KeyRetrievalService>();

build.Services.AddOptionsWithValidateOnStart<KeyRotationOptions>().BindConfiguration("KeyRotation")
    .ValidateDataAnnotations();
build.Services.AddHostedService<KeyRotationWorker>();

WebApplication app = build.Build();

await app.RunAsync();
