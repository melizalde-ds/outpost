using KeyProvider.Api;
using KeyProvider.Database;
using KeyProvider.KeyGeneration;
using KeyProvider.KeyRotation;
using KeyProvider.Signing;
using KeyProvider.SigningKeys;

using Microsoft.Extensions.Options;

using Polly;

WebApplicationBuilder build = WebApplication.CreateBuilder(args);

build.Services.AddSingleton(TimeProvider.System);

build.Services.AddKeyProviderDbContext(build.Configuration);
build.Services.AddDataProtectionDbContext(build.Configuration);

build.Services.AddSingleton<Ed25519KeyGenerator>();

build.Services.AddScoped<IKeyPairStore, KeyPairStore>();
build.Services.AddScoped<IRotationJobStore, RotationJobStore>();

build.Services.AddOptionsWithValidateOnStart<SigningOptions>().BindConfiguration("Signing")
    .ValidateDataAnnotations();
build.Services.AddSingleton<SigningService>();

build.Services.AddOptionsWithValidateOnStart<KeyRotationOptions>().BindConfiguration("KeyRotation")
    .ValidateDataAnnotations();
build.Services.AddSingleton<IKeyRotationService, KeyRotationService>();

build.Services.AddOptionsWithValidateOnStart<RotationRetryOptions>().BindConfiguration("KeyRotation:Retry")
    .ValidateDataAnnotations();
build.Services.AddResiliencePipeline<string, KeyRotationResult>(RotationRetryOptions.PipelineName,
    (builder, context) =>
    {
        RotationRetryOptions retry = context.ServiceProvider.GetRequiredService<IOptions<RotationRetryOptions>>()
            .Value;

        builder.TimeProvider = context.ServiceProvider.GetRequiredService<TimeProvider>();
        retry.ConfigurePipeline(builder);
    });

build.Services.AddOptionsWithValidateOnStart<RotationScheduleOptions>().BindConfiguration("KeyRotation:Schedule")
    .ValidateDataAnnotations();
build.Services.AddSingleton<IValidateOptions<RotationScheduleOptions>, RotationScheduleValidator>();

build.Services.AddHostedService<RotationWorker>();

build.Services.AddGrpc(o => o.Interceptors.Add<ErrorInterceptor>());

WebApplication app = build.Build();

app.MapGrpcService<KeyApiService>();

await app.RunAsync();
