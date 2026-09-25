using System.ComponentModel.DataAnnotations.Schema;

namespace KeyProvider.Repository;

public sealed class KeyRotationJob : IAppendOnly
{
    [Column("id")]
    [DatabaseGenerated(DatabaseGeneratedOption.None)]
    public required Guid RotationJobId { get; init; }
    public required DateTimeOffset StartedAt { get; init; }
    public required int Tries { get; init; }
    public required DateTimeOffset FinishedAt { get; init; }
    public required bool Success { get; init; }
}
