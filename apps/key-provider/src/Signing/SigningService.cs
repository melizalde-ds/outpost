using System.Buffers.Text;

using Microsoft.Extensions.Options;

using Org.BouncyCastle.Crypto.Parameters;

namespace KeyProvider.Signing;

internal sealed class SigningService(IOptions<SigningServiceOptions> options)
{
    private readonly SigningServiceOptions _options = options.Value;
    private Ed25519PrivateKeyParameters? _activePrivateKey;

    internal static string B64(ReadOnlySpan<byte> b)
    {
        return Base64Url.EncodeToString(b);
    }
}
