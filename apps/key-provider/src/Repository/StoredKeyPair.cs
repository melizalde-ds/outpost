using System.ComponentModel.DataAnnotations;
using System.ComponentModel.DataAnnotations.Schema;

using Microsoft.EntityFrameworkCore;

namespace KeyProvider.Repository;

public sealed class StoredKeyPair
{
    [Key]
    [DatabaseGenerated(DatabaseGeneratedOption.None)]
    public required Guid KeyId { get; init; }
    public required DateTimeOffset CreatedAt { get; init; }
    public required string PublicKey { get; init; }
    public required string PrivateKey { get; init; }
    public required DateTimeOffset ExpiresAt { get; init; }

    public required Guid RotationJobId { get; init; }

    [DeleteBehavior(DeleteBehavior.Restrict)]
    public required KeyRotationJob RotationJob { get; init; }
}
