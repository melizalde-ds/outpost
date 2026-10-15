namespace KeyProvider.KeyRotation;

internal interface IKeyRotationService
{
    Task<KeyRotationResult> RotateAsync(CancellationToken ct = default);

    Task<KeyRotationJob?> GetLatestSuccessfulJobAsync(CancellationToken ct);
}
