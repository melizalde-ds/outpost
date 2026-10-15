using KeyProvider.KeyRotation;
using KeyProvider.Signing;

using Microsoft.Extensions.Options;
using Microsoft.Extensions.Time.Testing;

using Shouldly;

namespace KeyProvider.UnitTests.KeyRotation;

public sealed class RotationScheduleValidatorTests
{
    private static readonly DateTimeOffset Now = new(2026, 1, 1, 0, 0, 0, TimeSpan.Zero);

    private static readonly RotationRetryOptions ProductionRetry = new()
    {
        MaxRetryAttempts = 3,
        BaseDelay = TimeSpan.FromSeconds(2),
        MaxDelay = TimeSpan.FromMinutes(1),
        AttemptTimeout = TimeSpan.FromSeconds(30)
    };

    private static readonly RotationRetryOptions DevelopmentRetry = new()
    {
        MaxRetryAttempts = 3,
        BaseDelay = TimeSpan.FromSeconds(1),
        MaxDelay = TimeSpan.FromSeconds(5),
        AttemptTimeout = TimeSpan.FromSeconds(10)
    };

    private static readonly TimeSpan MinimumProductionLifespan = new(8, 14, 30);

    private static ValidateOptionsResult Validate(
        RotationScheduleOptions schedule, TimeSpan lifespan, RotationRetryOptions retry)
    {
        RotationScheduleValidator validator = new(
            Options.Create(new KeyRotationOptions { Lifespan = lifespan }),
            Options.Create(retry),
            Options.Create(new SigningOptions()),
            new FakeTimeProvider(Now));

        return validator.Validate(null, schedule);
    }

    [Fact]
    public void Validate_ProductionSettings_Succeeds()
    {
        // Arrange
        RotationScheduleOptions schedule = new() { Expression = "0 */8 * * *" };

        // Act
        ValidateOptionsResult result = Validate(schedule, TimeSpan.FromDays(1), ProductionRetry);

        // Assert
        result.Succeeded.ShouldBeTrue();
    }

    [Fact]
    public void Validate_DevelopmentSettings_Succeeds()
    {
        // Arrange
        RotationScheduleOptions schedule = new()
        {
            Expression = "*/2 * * * *",
            MaxCheckDelay = TimeSpan.FromSeconds(30),
            FailureRetryDelay = TimeSpan.FromSeconds(20)
        };

        // Act
        ValidateOptionsResult result = Validate(schedule, TimeSpan.FromMinutes(15), DevelopmentRetry);

        // Assert
        result.Succeeded.ShouldBeTrue();
    }

    [Theory]
    [InlineData("")]
    [InlineData("not a schedule")]
    public void Validate_InvalidExpression_Fails(string expression)
    {
        // Arrange
        RotationScheduleOptions schedule = new() { Expression = expression };

        // Act
        ValidateOptionsResult result = Validate(schedule, TimeSpan.FromDays(1), ProductionRetry);

        // Assert
        result.Failed.ShouldBeTrue();
    }

    [Fact]
    public void Validate_KeysExpireBeforeTheNextRotation_Fails()
    {
        // Arrange
        RotationScheduleOptions schedule = new() { Expression = "0 */8 * * *" };

        // Act
        ValidateOptionsResult result = Validate(schedule, TimeSpan.FromHours(8), ProductionRetry);

        // Assert
        result.Failed.ShouldBeTrue();
    }

    [Fact]
    public void Validate_LifespanEqualToTheMinimum_Fails()
    {
        // Arrange
        RotationScheduleOptions schedule = new() { Expression = "0 */8 * * *" };

        // Act
        ValidateOptionsResult result = Validate(schedule, MinimumProductionLifespan, ProductionRetry);

        // Assert
        result.Failed.ShouldBeTrue();
    }

    [Fact]
    public void Validate_LifespanJustOverTheMinimum_Succeeds()
    {
        // Arrange
        RotationScheduleOptions schedule = new() { Expression = "0 */8 * * *" };

        // Act
        ValidateOptionsResult result =
            Validate(schedule, MinimumProductionLifespan + TimeSpan.FromSeconds(1), ProductionRetry);

        // Assert
        result.Succeeded.ShouldBeTrue();
    }
}
