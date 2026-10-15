using Google.Protobuf;

namespace Outpost.Key.V1;

public sealed partial class SignTokenResponse : ICustomDiagnosticMessage
{
    public string ToDiagnosticString()
    {
        SignTokenResponse redacted = Clone();
        redacted.Token = "[REDACTED]";
        return JsonFormatter.Default.Format(redacted);
    }
}
