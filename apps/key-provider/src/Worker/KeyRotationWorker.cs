using KeyProvider.Keys;

using Microsoft.Extensions.Options;

namespace KeyProvider.Worker;

internal sealed class KeyRotationWorker(IOptions<KeyRotationOptions> options, ILogger<KeyRotationWorker> logger, Ed25519KeyGenerator engine) : BackgroundService
{
    private readonly KeyRotationOptions _options = options.Value;
    private readonly Ed25519KeyGenerator _engine = engine;
    private readonly ILogger<KeyRotationWorker> _logger = logger;

    async protected override Task ExecuteAsync(CancellationToken stoppingToken)
    {
        this._logger.LogInformation("Worker started with interval {Interval}", _options.Interval);
        var keyPair = _engine.GenerateKeyPair();
        this._logger.LogInformation("Keypair generated");
    }
}
