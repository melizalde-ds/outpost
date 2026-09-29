namespace KeyProvider.Api;

public sealed class InvalidSubjectException(string subject)
    : Exception($"Subject '{subject}' is not a valid user ID.")
{
    public string Subject { get; } = subject;
}
