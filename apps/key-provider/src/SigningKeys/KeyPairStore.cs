using KeyProvider.Database;

using Microsoft.AspNetCore.DataProtection;
using Microsoft.EntityFrameworkCore;

namespace KeyProvider.SigningKeys;

internal sealed class KeyPairStore(
    KeyProviderDbContext context,
    TimeProvider timeProvider,
    IDataProtectionProvider dataProtectionProvider)
{
    private readonly IDataProtector _protector =
        dataProtectionProvider.CreateProtector(KeyPairRecord.PrivateKeyProtectionPurpose);

    internal async Task SaveRotationAsync(KeyPairRecord record, CancellationToken ct = default)
    {
        if (!record.RotationJob.Success)
        {
            throw new ArgumentException("A key pair can only be saved with a successful rotation job.",
                nameof(record));
        }

        context.KeyPairRecords.Add(record);

        await context.SaveChangesAsync(ct);
    }

    internal async Task<PrivateSigningKey?> GetLatestPrivateKeyAsync(CancellationToken ct = default)
    {
        KeyPairRecord? record = await context.KeyPairRecords
            .AsNoTracking()
            .NewestFirst()
            .FirstOrDefaultAsync(ct);

        if (record is null)
        {
            return null;
        }

        return new PrivateSigningKey
        {
            Id = record.KeyPairId,
            PrivateKey = _protector.Unprotect(record.PrivateKey),
            ExpirationTime = record.ExpiresAt
        };
    }

    internal Task<PublicSigningKey[]> GetVerificationKeysAsync(CancellationToken ct = default)
    {
        return context.KeyPairRecords
            .AsNoTracking()
            .Verifiable(timeProvider)
            .NewestFirst()
            .Select(x =>
                new PublicSigningKey { Id = x.KeyPairId, PublicKey = x.PublicKey, ExpirationTime = x.ExpiresAt })
            .ToArrayAsync(ct);
    }

    internal Task<PublicSigningKey?> GetVerificationKeyAsync(Guid id, CancellationToken ct = default)
    {
        return context.KeyPairRecords
            .AsNoTracking()
            .Verifiable(timeProvider)
            .Where(k => k.KeyPairId == id)
            .Select(x =>
                new PublicSigningKey { Id = x.KeyPairId, PublicKey = x.PublicKey, ExpirationTime = x.ExpiresAt })
            .FirstOrDefaultAsync(ct);
    }
}
