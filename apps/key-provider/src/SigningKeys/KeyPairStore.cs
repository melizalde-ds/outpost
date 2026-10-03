using System.Data.Common;
using System.Security.Cryptography;

using KeyProvider.Database;

using Microsoft.AspNetCore.DataProtection;
using Microsoft.EntityFrameworkCore;

using Org.BouncyCastle.Crypto.Parameters;

namespace KeyProvider.SigningKeys;

internal sealed class KeyPairStore(
    KeyProviderDbContext context,
    TimeProvider timeProvider,
    IDataProtectionProvider dataProtectionProvider)
{
    private readonly IDataProtector _protector =
        dataProtectionProvider.CreateProtector(KeyPairRecord.PrivateKeyProtectionPurpose);

    internal async Task SaveKeyPairAsync(KeyPairRecord record, CancellationToken ct = default)
    {
        if (!record.RotationJob.Success)
        {
            throw new ArgumentException("A key pair can only be saved with a successful rotation job.",
                nameof(record));
        }

        context.KeyPairRecords.Add(record);

        await context.SaveChangesAsync(ct);
    }

    internal async Task<SigningKeyPair?> GetLatestKeyPairAsync(CancellationToken ct = default)
    {
        KeyPairRecord? record;
        try
        {
            record = await context.KeyPairRecords
                .AsNoTracking()
                .NewestFirst()
                .FirstOrDefaultAsync(ct);
        }
        catch (Exception e) when (IsTemporary(e))
        {
            throw new DatabaseUnavailableException(e);
        }

        if (record is null)
        {
            return null;
        }

        byte[] privateKeyBytes;
        try
        {
            privateKeyBytes = _protector.Unprotect(record.PrivateKey);
        }
        catch (CryptographicException e)
        {
            throw new SigningKeyUndecryptableException(record.KeyPairId, e);
        }

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

    internal async Task<PublicSigningKey[]> GetVerificationKeysAsync(CancellationToken ct = default)
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
        catch (Exception e) when (IsTemporary(e))
        {
            throw new DatabaseUnavailableException(e);
        }
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

    private static bool IsTemporary(Exception e)
    {
        for (Exception? current = e; current is not null; current = current.InnerException)
        {
            if (current is TimeoutException or DbException { IsTransient: true })
            {
                return true;
            }
        }

        return false;
    }
}
