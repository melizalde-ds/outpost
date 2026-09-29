using KeyProvider.Data;
using KeyProvider.Models;

using Microsoft.AspNetCore.DataProtection;
using Microsoft.EntityFrameworkCore;

namespace KeyProvider.KeyPairHub;

internal sealed class KeyRetrievalService(
    KeyProviderDbContext context,
    TimeProvider timeProvider,
    IDataProtectionProvider dataProtectionProvider)
{
    private readonly IDataProtector _protector =
        dataProtectionProvider.CreateProtector(KeyPairRecord.PrivateKeyProtectionPurpose);

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

    internal Task<KeyRotationJob?> GetRotationJobAsync(CancellationToken ct = default)
    {
        return context.KeyRotationJobs
            .AsNoTracking()
            .LatestSuccessfulFirst()
            .FirstOrDefaultAsync(ct);
    }
}
