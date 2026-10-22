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

    private static readonly TimeSpan MinimumProductionLifespan = new(8, 18, 0);

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

    [Fact]
    public void Validate_DefaultSchedule_Succeeds()
    {
        // Arrange
        RotationScheduleOptions schedule = new();

        // Act
        ValidateOptionsResult result = Validate(schedule, TimeSpan.FromDays(1), ProductionRetry);

        // Assert
        result.Succeeded.ShouldBeTrue();
    }

    [Fact]
    public void Validate_EmptyExpression_FailsSayingItIsRequired()
    {
        // Arrange
        RotationScheduleOptions schedule = new() { Expression = "" };

        // Act
        ValidateOptionsResult result = Validate(schedule, TimeSpan.FromDays(1), ProductionRetry);

        // Assert
        result.FailureMessage.ShouldBe("KeyRotation:Schedule:Expression is required.");
    }

    [Fact]
    public void Validate_NotACronExpression_FailsNamingTheExpression()
    {
        // Arrange
        RotationScheduleOptions schedule = new() { Expression = "not a schedule" };

        // Act
        ValidateOptionsResult result = Validate(schedule, TimeSpan.FromDays(1), ProductionRetry);

        // Assert
        result.FailureMessage.ShouldBe("'not a schedule' is not a valid cron expression.");
    }

    [Fact]
    public void Validate_ExpressionThatNeverFires_FailsSayingItMustRepeat()
    {
        // Arrange
        RotationScheduleOptions schedule = new() { Expression = "0 0 30 2 *" };

        // Act
        ValidateOptionsResult result = Validate(schedule, TimeSpan.FromDays(1), ProductionRetry);

        // Assert
        result.FailureMessage.ShouldBe("'0 0 30 2 *' must fire repeatedly.");
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
    public void Validate_KeysExpireBeforeTheNextRotation_FailsNamingEachPart()
    {
        // Arrange
        RotationScheduleOptions schedule = new() { Expression = "0 */8 * * *" };

        // Act
        ValidateOptionsResult result = Validate(schedule, TimeSpan.FromHours(8), ProductionRetry);

        // Assert
        result.FailureMessage.ShouldBe(
            "KeyRotation:Lifespan (08:00:00) must be longer than the largest gap between schedule slots " +
            "(08:00:00) plus KeyRotation:Schedule:FailureRetryDelay (00:05:00) plus the " +
            "worst-case retry duration (00:05:00) plus Signing:TokenLifetime (00:05:00) plus " +
            "Signing:PublishDelay (00:03:00).");
    }

    [Fact]
    public void Validate_YearlySlotExactlyOneYearAway_Succeeds()
    {
        // Arrange
        RotationScheduleOptions schedule = new() { Expression = "0 0 1 1 *" };

        // Act
        ValidateOptionsResult result = Validate(schedule, TimeSpan.FromDays(400), ProductionRetry);

        // Assert
        result.Succeeded.ShouldBeTrue();
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

    [Fact]
    public void Validate_ScheduleWithALongGapAfterManyQuickSlots_Fails()
    {
        // Arrange
        RotationScheduleOptions schedule = new() { Expression = "* 0 * * *" };

        // Act
        ValidateOptionsResult result = Validate(schedule, TimeSpan.FromHours(1), ProductionRetry);

        // Assert
        result.Failed.ShouldBeTrue();
    }
}
