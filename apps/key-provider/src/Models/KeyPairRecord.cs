using System.ComponentModel.DataAnnotations;
using System.ComponentModel.DataAnnotations.Schema;

using Weatherlab.EntityFrameworkCore.AppendOnly;
using Weatherlab.EntityFrameworkCore.CreatedAt;

namespace KeyProvider.Models;

[AppendOnly]
[Table("key_pair_record")]
internal sealed class KeyPairRecord
{
    [Key]
    [Column("id")]
    [DatabaseGenerated(DatabaseGeneratedOption.None)]
    public required Guid KeyPairId { get; init; }

    [CreatedAt] public DateTimeOffset CreatedAt { get; init; }

    public required string PublicKey { get; init; }
    public required string PrivateKey { get; init; }
    public required DateTimeOffset ExpiresAt { get; init; }

    public required Guid KeyPairRotationJobId { get; init; }

    [ForeignKey(nameof(KeyPairRotationJobId))]
    public required KeyRotationJob RotationJob { get; init; }
}
