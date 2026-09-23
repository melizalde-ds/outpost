using Key.Engine;

using Microsoft.Extensions.Options;

namespace Key.Worker;

internal sealed class Worker(IOptions<WorkerOptions> options, ILogger<Worker> logger, ED25519Engine engine) : BackgroundService
{
    private readonly WorkerOptions _options = options.Value;
    private readonly ED25519Engine _engine = engine;
    private readonly ILogger<Worker> _logger = logger;

    async protected override Task ExecuteAsync(CancellationToken stoppingToken)
    {
        this._logger.LogInformation("Worker started with interval {Interval}", _options.Interval);
        var keyPair = _engine.GenerateKeyPair();
        this._logger.LogInformation("Keypair generated");
    }
}
