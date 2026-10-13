using System.ComponentModel.DataAnnotations;
using System.ComponentModel.DataAnnotations.Schema;

using Outpost.EntityFrameworkCore.AppendOnly;
using Outpost.EntityFrameworkCore.CreatedAt;

namespace KeyProvider.KeyRotation;

[AppendOnly]
[Table("key_rotation_job")]
internal sealed class KeyRotationJob
{
    [Key]
    [Column("id")]
    [DatabaseGenerated(DatabaseGeneratedOption.None)]
    public required Guid KeyPairRotationJobId { get; init; }

    public required DateTimeOffset StartedAt { get; init; }

    [CreatedAt] public DateTimeOffset FinishedAt { get; init; }

    public required bool Success { get; init; }

    [MaxLength(512)] public string? ErrorMessage { get; init; }
}
