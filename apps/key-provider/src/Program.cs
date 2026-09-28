using KeyProvider.CryptoKeys;
using KeyProvider.Data;
using KeyProvider.KeyPairHub;
using KeyProvider.KeyRotation;
using KeyProvider.Worker;

using Microsoft.Extensions.Options;

using Polly;
using Polly.Retry;
using Polly.Timeout;

WebApplicationBuilder build = WebApplication.CreateBuilder(args);

build.Services.AddSingleton(TimeProvider.System);

build.Services.AddKeyProviderDbContext(build.Configuration);

build.Services.AddSingleton<Ed25519KeyGenerator>();

build.Services.AddScoped<KeyIngestionService>();
build.Services.AddScoped<KeyRetrievalService>();

build.Services.AddOptionsWithValidateOnStart<KeyRotationOptions>().BindConfiguration("KeyRotation")
    .ValidateDataAnnotations();
build.Services.AddSingleton<KeyRotationService>();

build.Services.AddOptionsWithValidateOnStart<RotationRetryOptions>().BindConfiguration("RotationRetry")
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

build.Services.AddOptionsWithValidateOnStart<WorkerOptions>().BindConfiguration("Worker")
    .ValidateDataAnnotations();
build.Services.AddSingleton<IValidateOptions<WorkerOptions>, WorkerOptionsValidator>();

build.Services.AddHostedService<RotationWorker>();

WebApplication app = build.Build();

await app.RunAsync();
