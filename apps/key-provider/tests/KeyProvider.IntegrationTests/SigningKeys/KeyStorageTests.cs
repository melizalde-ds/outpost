using Grpc.Core;
using KeyProvider.Database;
using KeyProvider.IntegrationTests.Infrastructure;
using KeyProvider.KeyRotation;
using KeyProvider.SigningKeys;
using KeyProvider.UnitTests;
using Microsoft.AspNetCore.DataProtection;
using Microsoft.Extensions.DependencyInjection;
using Npgsql;
using Org.BouncyCastle.Crypto.Parameters;
using Outpost.EntityFrameworkCore.AppendOnly;
using Outpost.Key.V1;
using Shouldly;

namespace KeyProvider.IntegrationTests.SigningKeys;

public sealed class KeyStorageTests(PostgresFixture postgres)
{
    private const string Subject = "00000000-0000-0000-0000-000000000001";

    private readonly CancellationToken _ct = TestContext.Current.CancellationToken;

    private async Task<NpgsqlConnection> OpenAdminAsync(TestDatabases databases)
    {
        NpgsqlConnection admin = new(databases.KeyProviderAdmin);
        await admin.OpenAsync(_ct);
        return admin;
    }

    [Fact]
    public async Task SignToken_KeyMadeBeforeARestart_StillSigns()
    {
        // Arrange
        TestDatabases databases = await postgres.CreateDatabasesAsync(_ct);
        Guid keyId;
        await using (ServiceFactory before = new(databases))
        {
            keyId = await before.RotateAsync(_ct);
        }

        await using ServiceFactory after = new(databases);

        // Act
        SignTokenResponse response = await after
            .CreateKeyClient()
            .SignTokenAsync(new SignTokenRequest { Subject = Subject }, cancellationToken: _ct);

        // Assert
        response.KeyId.ShouldBe(keyId.ToString());
        TestKeys
            .Verify(response.Token, response.VerificationKey.PublicKey.ToByteArray())
            .ShouldBeTrue();
    }

    [Fact]
    public async Task RotateAsync_NewKey_StoresThePrivateKeyEncrypted()
    {
        // Arrange
        TestDatabases databases = await postgres.CreateDatabasesAsync(_ct);
        await using ServiceFactory service = new(databases);
        await service.RotateAsync(_ct);
        await using NpgsqlConnection admin = await OpenAdminAsync(databases);
        await using NpgsqlCommand select = new(
            "SELECT private_key, public_key FROM key_pair_record",
            admin
        );

        // Act
        await using NpgsqlDataReader row = await select.ExecuteReaderAsync(_ct);

        // Assert
        await row.ReadAsync(_ct);
        byte[] stored = (byte[])row["private_key"];
        byte[] publicKey = (byte[])row["public_key"];
        byte[] privateKey = service
            .Services.GetRequiredService<IDataProtectionProvider>()
            .CreateProtector(KeyPairRecord.PrivateKeyProtectionPurpose)
            .Unprotect(stored);
        new Ed25519PrivateKeyParameters(privateKey)
            .GeneratePublicKey()
            .GetEncoded()
            .ShouldBe(publicKey);
        stored.AsSpan().IndexOf(privateKey).ShouldBe(-1);
    }

    [Fact]
    public async Task RotateAsync_NewKey_SavesTheEncryptionKeyInTheDatabase()
    {
        // Arrange
        TestDatabases databases = await postgres.CreateDatabasesAsync(_ct);
        await using ServiceFactory service = new(databases);
        await service.RotateAsync(_ct);
        await using NpgsqlConnection dataProtection = new(databases.DataProtection);
        await dataProtection.OpenAsync(_ct);
        await using NpgsqlCommand count = new(
            """SELECT count(*) FROM "DataProtectionKeys" """,
            dataProtection
        );

        // Act
        object? keyCount = await count.ExecuteScalarAsync(_ct);

        // Assert
        keyCount.ShouldBe(1L);
    }

    [Theory]
    [InlineData("UPDATE key_pair_record SET expires_at = expires_at")]
    [InlineData("DELETE FROM key_pair_record")]
    [InlineData("TRUNCATE key_pair_record")]
    [InlineData("UPDATE key_rotation_job SET finished_at = finished_at")]
    [InlineData("DELETE FROM key_rotation_job")]
    [InlineData("TRUNCATE key_rotation_job CASCADE")]
    public async Task KeyHistory_Changing_IsRefusedByTheDatabase(string sql)
    {
        // Arrange
        TestDatabases databases = await postgres.CreateDatabasesAsync(_ct);
        await using (ServiceFactory service = new(databases))
        {
            await service.RotateAsync(_ct);
        }

        await using NpgsqlConnection admin = await OpenAdminAsync(databases);
        // The SQL comes from the test cases above, not from input.
#pragma warning disable CA2100
        await using NpgsqlCommand change = new(sql, admin);
#pragma warning restore CA2100

        // Act
        PostgresException error = await Should.ThrowAsync<PostgresException>(() =>
            change.ExecuteNonQueryAsync(_ct)
        );

        // Assert
        error.SqlState.ShouldBe(AppendOnlySqlState.Violation);
    }

    [Fact]
    public async Task SaveKeyPairAsync_RotationJobFailed_Throws()
    {
        // Arrange
        await using ServiceFactory service = new(await postgres.CreateDatabasesAsync(_ct));
        await using AsyncServiceScope scope = service.Services.CreateAsyncScope();
        IKeyPairStore store = scope.ServiceProvider.GetRequiredService<IKeyPairStore>();
        KeyRotationJob failedJob = new()
        {
            KeyPairRotationJobId = Guid.CreateVersion7(),
            StartedAt = ServiceFactory.Start,
            Success = false,
            ErrorMessage = "failed",
        };
        KeyPairRecord record = new()
        {
            KeyPairId = Guid.CreateVersion7(),
            PublicKey = new byte[Ed25519PublicKeyParameters.KeySize],
            PrivateKey = [0],
            ExpiresAt = ServiceFactory.Start.AddDays(1),
            KeyPairRotationJobId = failedJob.KeyPairRotationJobId,
            RotationJob = failedJob,
        };

        // Act
        ArgumentException error = await Should.ThrowAsync<ArgumentException>(() =>
            store.SaveKeyPairAsync(record, _ct)
        );

        // Assert
        error.ParamName.ShouldBe("record");
        error.Message.ShouldStartWith(
            "A key pair can only be saved with a successful rotation job."
        );
    }

    [Fact]
    public async Task GetLatestKeyPairAsync_DatabaseUnreachable_ThrowsDatabaseUnavailable()
    {
        // Arrange
        TestDatabases databases = await postgres.CreateDatabasesAsync(_ct);
        string closedPort = new NpgsqlConnectionStringBuilder(databases.KeyProvider)
        {
            Host = "127.0.0.1",
            Port = 1,
        }.ConnectionString;
        await using ServiceFactory service = new(databases with { KeyProvider = closedPort });
        await using AsyncServiceScope scope = service.Services.CreateAsyncScope();
        IKeyPairStore store = scope.ServiceProvider.GetRequiredService<IKeyPairStore>();

        // Act
        Exception error = await Should.ThrowAsync<Exception>(() =>
            store.GetLatestKeyPairAsync(_ct)
        );

        // Assert
        error.ShouldBeOfType<DatabaseUnavailableException>();
        error.Message.ShouldBe("The database is temporarily unavailable.");
    }

    [Fact]
    public async Task SignToken_DatabaseRefusesAccess_ReturnsInternal()
    {
        // Arrange
        TestDatabases databases = await postgres.CreateDatabasesAsync(_ct);
        await using NpgsqlConnection admin = await OpenAdminAsync(databases);
        await using NpgsqlCommand revoke = new(
            "REVOKE SELECT ON key_pair_record FROM key_provider_app",
            admin
        );
        await revoke.ExecuteNonQueryAsync(_ct);
        await using ServiceFactory service = new(databases);

        // Act
        RpcException error = await Should.ThrowAsync<RpcException>(async () =>
            await service
                .CreateKeyClient()
                .SignTokenAsync(new SignTokenRequest { Subject = Subject }, cancellationToken: _ct)
        );

        // Assert
        error.StatusCode.ShouldBe(StatusCode.Internal);
    }

    [Fact]
    public async Task KeyPairRecord_RotationJobFailed_IsRefusedByTheDatabase()
    {
        // Arrange
        TestDatabases databases = await postgres.CreateDatabasesAsync(_ct);
        await using NpgsqlConnection admin = await OpenAdminAsync(databases);
        await using NpgsqlCommand failedJob = new(
            """
            INSERT INTO key_rotation_job (id, started_at, finished_at, success, error_message)
            VALUES ('00000000-0000-0000-0000-0000000000b1', now(), now(), false, 'failed')
            """,
            admin
        );
        await failedJob.ExecuteNonQueryAsync(_ct);
        await using NpgsqlCommand key = new(
            """
            INSERT INTO key_pair_record (id, created_at, public_key, private_key, expires_at, key_pair_rotation_job_id)
            VALUES ('00000000-0000-0000-0000-0000000000a1', now(), decode(repeat('00', 32), 'hex'), '\x00',
                    now() + interval '1 day', '00000000-0000-0000-0000-0000000000b1')
            """,
            admin
        );

        // Act
        PostgresException error = await Should.ThrowAsync<PostgresException>(() =>
            key.ExecuteNonQueryAsync(_ct)
        );

        // Assert
        error.SqlState.ShouldBe(DatabaseErrorCodes.RotationJobNotSuccessful);
    }
}
