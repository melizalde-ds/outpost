using KeyProvider.Keys;

using Microsoft.Extensions.Options;

using Org.BouncyCastle.Crypto.Parameters;

namespace KeyProvider.Worker;

internal sealed class KeyRotationWorker(
    IOptions<KeyRotationOptions> options,
    ILogger<KeyRotationWorker> logger,
    Ed25519KeyGenerator engine) : BackgroundService
{
    private readonly Ed25519KeyGenerator _engine = engine;
    private readonly ILogger<KeyRotationWorker> _logger = logger;
    private readonly KeyRotationOptions _options = options.Value;

    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
    {
        _logger.LogInformation("Worker started with interval {Interval}", _options.Interval);
        AsymmetricKeyPair<Ed25519PrivateKeyParameters, Ed25519PublicKeyParameters> keyPair = _engine.GenerateKeyPair();
        _logger.LogInformation("Keypair generated");
    }
}
