namespace KeyProvider.KeyRotation;

internal interface IRotationJobStore
{
    Task SaveFailedRotationAsync(KeyRotationJob job, CancellationToken ct = default);

    Task<KeyRotationJob?> GetLatestSuccessfulJobAsync(CancellationToken ct = default);
}
