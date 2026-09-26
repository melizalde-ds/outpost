using System.ComponentModel.DataAnnotations;
using System.ComponentModel.DataAnnotations.Schema;

using Weatherlab.EntityFrameworkCore.AppendOnly;
using Weatherlab.EntityFrameworkCore.CreatedAt;

namespace KeyProvider.Models;

[AppendOnly]
[Table("key_rotation_job")]
internal sealed class KeyRotationJob
{
    [Key]
    [Column("id")]
    [DatabaseGenerated(DatabaseGeneratedOption.None)]
    public required Guid KeyPairRotationJobId { get; init; }

    public required DateTimeOffset StartedAt { get; init; }
    public required int Tries { get; init; }

    [CreatedAt] public DateTimeOffset FinishedAt { get; init; }

    public required bool Success { get; init; }
}
