using System.ComponentModel.DataAnnotations;
using System.ComponentModel.DataAnnotations.Schema;

using Microsoft.EntityFrameworkCore;

using Weatherlab.EntityFrameworkCore.AppendOnly;
using Weatherlab.EntityFrameworkCore.CreatedAt;

namespace KeyProvider.Models;

[AppendOnly]
[Table("key_pair_record")]
[Index(nameof(KeyPairRotationJobId), IsUnique = true)]
[Index(nameof(CreatedAt), IsDescending = [true])]
[Index(nameof(ExpiresAt))]
internal sealed class KeyPairRecord
{
    internal const string PrivateKeyProtectionPurpose = "KeyProvider.KeyPairRecord.PrivateKey";

    [Key]
    [Column("id")]
    [DatabaseGenerated(DatabaseGeneratedOption.None)]
    public required Guid KeyPairId { get; init; }

    [CreatedAt] public DateTimeOffset CreatedAt { get; init; }

    public required byte[] PublicKey { get; init; }
    public required byte[] PrivateKey { get; init; }
    public required DateTimeOffset ExpiresAt { get; init; }

    public required Guid KeyPairRotationJobId { get; init; }

    [ForeignKey(nameof(KeyPairRotationJobId))]
    public required KeyRotationJob RotationJob { get; init; }
}
