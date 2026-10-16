using KeyProvider.IntegrationTests.Infrastructure;
using Outpost.Key.V1;
using Shouldly;

namespace KeyProvider.IntegrationTests.Api;

[Trait("Consumer", "auth")]
public sealed class GetVerificationKeysTests(PostgresFixture postgres)
{
    [Fact]
    public async Task GetVerificationKeys_NoKeys_ReturnsEmpty()
    {
        // Arrange
        CancellationToken ct = TestContext.Current.CancellationToken;
        await using ServiceFactory service = new(await postgres.CreateDatabasesAsync(ct));
        KeyService.KeyServiceClient client = service.CreateKeyClient();

        // Act
        GetVerificationKeysResponse response = await client.GetVerificationKeysAsync(
            new GetVerificationKeysRequest(),
            cancellationToken: ct
        );

        // Assert
        response.Keys.ShouldBeEmpty();
    }
}
