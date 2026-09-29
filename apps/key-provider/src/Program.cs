using KeyProvider.Api;
using KeyProvider.Database;
using KeyProvider.KeyGeneration;
using KeyProvider.KeyRotation;
using KeyProvider.Signing;
using KeyProvider.SigningKeys;

using Microsoft.Extensions.Options;

using Polly;
using Polly.Retry;
using Polly.Timeout;

WebApplicationBuilder build = WebApplication.CreateBuilder(args);

build.Services.AddSingleton(TimeProvider.System);

build.Services.AddKeyProviderDbContext(build.Configuration);
build.Services.AddDataProtectionDbContext(build.Configuration);

build.Services.AddSingleton<Ed25519KeyGenerator>();

build.Services.AddScoped<KeyPairStore>();
build.Services.AddScoped<RotationJobStore>();

build.Services.AddOptionsWithValidateOnStart<SigningOptions>().BindConfiguration("Signing")
    .ValidateDataAnnotations();
build.Services.AddSingleton<SigningService>();

build.Services.AddOptionsWithValidateOnStart<KeyRotationOptions>().BindConfiguration("KeyRotation")
    .ValidateDataAnnotations();
build.Services.AddSingleton<KeyRotationService>();

build.Services.AddOptionsWithValidateOnStart<RotationRetryOptions>().BindConfiguration("KeyRotation:Retry")
    .ValidateDataAnnotations();
build.Services.AddResiliencePipeline<string, KeyRotationResult>(RotationRetryOptions.PipelineName,
    (builder, context) =>
    {
        RotationRetryOptions retry = context.ServiceProvider.GetRequiredService<IOptions<RotationRetryOptions>>()
            .Value;

        builder.TimeProvider = context.ServiceProvider.GetRequiredService<TimeProvider>();

        builder
            .AddRetry(new RetryStrategyOptions<KeyRotationResult>
            {
                ShouldHandle = new PredicateBuilder<KeyRotationResult>()
                    .HandleResult(r => r is KeyRotationFailed { IsTransient: true })
                    .Handle<TimeoutRejectedException>(),
                MaxRetryAttempts = retry.MaxRetryAttempts,
                BackoffType = DelayBackoffType.Exponential,
                UseJitter = true,
                Delay = retry.BaseDelay,
                MaxDelay = retry.MaxDelay
            })
            .AddTimeout(retry.AttemptTimeout);
    });

build.Services.AddOptionsWithValidateOnStart<RotationScheduleOptions>().BindConfiguration("KeyRotation:Schedule")
    .ValidateDataAnnotations();
build.Services.AddSingleton<IValidateOptions<RotationScheduleOptions>, RotationScheduleValidator>();

build.Services.AddHostedService<RotationWorker>();

build.Services.AddGrpc(o => o.Interceptors.Add<ErrorInterceptor>());

WebApplication app = build.Build();

app.MapGrpcService<KeyApiService>();

await app.RunAsync();
