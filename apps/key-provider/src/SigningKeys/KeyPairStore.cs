using System.Security.Cryptography;

using KeyProvider.Database;

using Microsoft.AspNetCore.DataProtection;
using Microsoft.EntityFrameworkCore;

using Org.BouncyCastle.Crypto.Parameters;

namespace KeyProvider.SigningKeys;

internal sealed class KeyPairStore(
    KeyProviderDbContext context,
    TimeProvider timeProvider,
    IDataProtectionProvider dataProtectionProvider) : IKeyPairStore
{
    private readonly IDataProtector _protector =
        dataProtectionProvider.CreateProtector(KeyPairRecord.PrivateKeyProtectionPurpose);

    public async Task SaveKeyPairAsync(KeyPairRecord record, CancellationToken ct = default)
    {
        if (!record.RotationJob.Success)
        {
            throw new ArgumentException("A key pair can only be saved with a successful rotation job.",
                nameof(record));
        }

        context.KeyPairRecords.Add(record);

        await context.SaveChangesAsync(ct);
    }

    public Task<SigningKeyPair?> GetLatestKeyPairAsync(CancellationToken ct = default)
    {
        return FindNewestAsync(context.KeyPairRecords, ct);
    }

    public Task<SigningKeyPair?> GetLatestKeyPairCreatedByAsync(DateTimeOffset createdBy,
        CancellationToken ct = default)
    {
        return FindNewestAsync(context.KeyPairRecords.CreatedBy(createdBy), ct);
    }

    private async Task<SigningKeyPair?> FindNewestAsync(IQueryable<KeyPairRecord> records, CancellationToken ct)
    {
        KeyPairRecord? record = await FindNewestRecordAsync(records, ct);

        return record is null ? null : Decode(record, _protector);
    }

    private static async Task<KeyPairRecord?> FindNewestRecordAsync(IQueryable<KeyPairRecord> records,
        CancellationToken ct)
    {
        try
        {
            return await records
                .AsNoTracking()
                .NewestFirst()
                .FirstOrDefaultAsync(ct);
        }
        catch (Exception e) when (DatabaseErrors.IsTemporary(e))
        {
            throw new DatabaseUnavailableException(e);
        }
    }

    internal static SigningKeyPair Decode(KeyPairRecord record, IDataProtector protector)
    {
        byte[] privateKeyBytes = Unprotect(record, protector);

        if (privateKeyBytes.Length != Ed25519PrivateKeyParameters.KeySize)
        {
            CryptographicOperations.ZeroMemory(privateKeyBytes);
            throw new SigningKeyMalformedException(record.KeyPairId, privateKeyBytes.Length,
                Ed25519PrivateKeyParameters.KeySize);
        }

        PrivateSigningKey privateKey = new()
        {
            Id = record.KeyPairId, PrivateKey = privateKeyBytes, ExpirationTime = record.ExpiresAt
        };

        PublicSigningKey publicKey = new()
        {
            Id = record.KeyPairId, PublicKey = record.PublicKey, ExpirationTime = record.ExpiresAt
        };

        return new SigningKeyPair(privateKey, publicKey);
    }

    private static byte[] Unprotect(KeyPairRecord record, IDataProtector protector)
    {
        try
        {
            return protector.Unprotect(record.PrivateKey);
        }
        catch (CryptographicException e)
        {
            throw new SigningKeyUndecryptableException(record.KeyPairId, e);
        }
    }

    public async Task<PublicSigningKey[]> GetVerificationKeysAsync(CancellationToken ct = default)
    {
        try
        {
            return await context.KeyPairRecords
                .AsNoTracking()
                .Verifiable(timeProvider)
                .OldestFirst()
                .Select(x =>
                    new PublicSigningKey { Id = x.KeyPairId, PublicKey = x.PublicKey, ExpirationTime = x.ExpiresAt })
                .ToArrayAsync(ct);
        }
        catch (Exception e) when (DatabaseErrors.IsTemporary(e))
        {
            throw new DatabaseUnavailableException(e);
        }
    }
}
