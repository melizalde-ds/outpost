using Microsoft.Extensions.Options;

namespace KeyProvider.KeyPairServices;

public sealed class KeyIngestionService(IOptions<KeyIngestionOptions> options)
{
    private readonly KeyIngestionOptions _options = options.Value;
}
