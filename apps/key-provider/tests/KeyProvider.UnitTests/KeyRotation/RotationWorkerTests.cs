using KeyProvider.KeyRotation;

using Microsoft.Extensions.Logging.Abstractions;
using Microsoft.Extensions.Options;
using Microsoft.Extensions.Time.Testing;

using NSubstitute;

using Polly;
using Polly.Registry;
using Polly.Timeout;

using Shouldly;

namespace KeyProvider.UnitTests.KeyRotation;

public sealed class RotationWorkerTests
{
    private const string EveryEightHours = "0 */8 * * *";
    private const string NeverFires = "0 0 30 2 *"; // February 30th
    private static readonly DateTimeOffset Now = new(2026, 1, 1, 0, 0, 0, TimeSpan.Zero);
    private static readonly TimeSpan MaxCheckDelay = TimeSpan.FromHours(1);
    private static readonly TimeSpan FailureRetryDelay = TimeSpan.FromMinutes(10);

    private readonly IKeyRotationService _rotation = Substitute.For<IKeyRotationService>();
    private readonly FakeTimeProvider _clock = new(Now);

    public RotationWorkerTests()
    {
        RotationReturns(new KeyRotationSucceeded(Now, Now, Guid.Empty));
    }

    private RotationWorker CreateWorker(string expression = EveryEightHours, TimeSpan? failureRetryDelay = null)
    {
        ResiliencePipelineProvider<string> pipelines = Substitute.For<ResiliencePipelineProvider<string>>();
        pipelines.GetPipeline<KeyRotationResult>(Arg.Any<string>())
            .Returns(ResiliencePipeline<KeyRotationResult>.Empty);

        return new RotationWorker(
            _rotation,
            pipelines,
            Options.Create(new RotationScheduleOptions
            {
                Expression = expression,
                MaxCheckDelay = MaxCheckDelay,
                FailureRetryDelay = failureRetryDelay ?? FailureRetryDelay
            }),
            NullLogger<RotationWorker>.Instance,
            _clock);
    }

    private void LastRotationFinishedAt(DateTimeOffset finishedAt)
    {
        _rotation.GetLatestSuccessfulJobAsync(Arg.Any<CancellationToken>()).Returns(new KeyRotationJob
        {
            KeyPairRotationJobId = Guid.Empty,
            StartedAt = finishedAt,
            FinishedAt = finishedAt,
            Success = true
        });
    }

    private void RotationReturns(KeyRotationResult result)
    {
        _rotation.RotateAsync(Arg.Any<CancellationToken>()).Returns(result);
    }

    [Fact]
    public async Task RunOnceAsync_NoRotationYet_Rotates()
    {
        // Arrange
        RotationWorker sut = CreateWorker();

        // Act
        await sut.RunOnceAsync(TestContext.Current.CancellationToken);

        // Assert
        await _rotation.Received(1).RotateAsync(Arg.Any<CancellationToken>());
    }

    [Fact]
    public async Task RunOnceAsync_LastRotationBeforeTheCurrentSlot_Rotates()
    {
        // Arrange
        LastRotationFinishedAt(Now.AddMinutes(-1));
        RotationWorker sut = CreateWorker();

        // Act
        await sut.RunOnceAsync(TestContext.Current.CancellationToken);

        // Assert
        await _rotation.Received(1).RotateAsync(Arg.Any<CancellationToken>());
    }

    [Fact]
    public async Task RunOnceAsync_LastRotationInTheCurrentSlot_DoesNotRotate()
    {
        // Arrange
        LastRotationFinishedAt(Now);
        RotationWorker sut = CreateWorker();

        // Act
        await sut.RunOnceAsync(TestContext.Current.CancellationToken);

        // Assert
        await _rotation.DidNotReceive().RotateAsync(Arg.Any<CancellationToken>());
    }

    [Fact]
    public async Task RunOnceAsync_NextSlotFurtherAwayThanMaxCheckDelay_WaitsMaxCheckDelay()
    {
        // Arrange
        LastRotationFinishedAt(Now);
        RotationWorker sut = CreateWorker();

        // Act
        TimeSpan wait = await sut.RunOnceAsync(TestContext.Current.CancellationToken);

        // Assert
        wait.ShouldBe(MaxCheckDelay);
    }

    [Fact]
    public async Task RunOnceAsync_NextSlotSoonerThanMaxCheckDelay_WaitsUntilTheSlot()
    {
        // Arrange
        LastRotationFinishedAt(Now);
        _clock.SetUtcNow(Now.AddHours(7.5));
        RotationWorker sut = CreateWorker();

        // Act
        TimeSpan wait = await sut.RunOnceAsync(TestContext.Current.CancellationToken);

        // Assert
        wait.ShouldBe(TimeSpan.FromMinutes(30));
    }

    [Fact]
    public async Task RunOnceAsync_RotationSucceeds_WaitsUntilTheNextSlot()
    {
        // Arrange
        _clock.SetUtcNow(Now.AddHours(7.5));
        RotationWorker sut = CreateWorker();

        // Act
        TimeSpan wait = await sut.RunOnceAsync(TestContext.Current.CancellationToken);

        // Assert
        wait.ShouldBe(TimeSpan.FromMinutes(30));
    }

    [Fact]
    public async Task RunOnceAsync_RotationFails_WaitsFailureRetryDelay()
    {
        // Arrange
        RotationReturns(new KeyRotationFailed(Now, Now, "failed", IsTransient: false));
        RotationWorker sut = CreateWorker();

        // Act
        TimeSpan wait = await sut.RunOnceAsync(TestContext.Current.CancellationToken);

        // Assert
        wait.ShouldBe(FailureRetryDelay);
    }

    [Fact]
    public async Task RunOnceAsync_RotationTimesOutOnEveryAttempt_WaitsFailureRetryDelay()
    {
        // Arrange
        _rotation.RotateAsync(Arg.Any<CancellationToken>())
            .Returns(Task.FromException<KeyRotationResult>(new TimeoutRejectedException()));
        RotationWorker sut = CreateWorker();

        // Act
        TimeSpan wait = await sut.RunOnceAsync(TestContext.Current.CancellationToken);

        // Assert
        wait.ShouldBe(FailureRetryDelay);
    }

    [Fact]
    public async Task RunOnceAsync_ScheduleHasNoUpcomingSlot_WaitsMaxCheckDelay()
    {
        // Arrange
        LastRotationFinishedAt(Now);
        RotationWorker sut = CreateWorker(expression: NeverFires);

        // Act
        TimeSpan wait = await sut.RunOnceAsync(TestContext.Current.CancellationToken);

        // Assert
        wait.ShouldBe(MaxCheckDelay);
    }

    [Fact]
    public async Task StartAsync_CheckThrows_KeepsRunningAndChecksAgain()
    {
        // Arrange
        TaskCompletionSource secondCheck = new(TaskCreationOptions.RunContinuationsAsynchronously);
        _rotation.GetLatestSuccessfulJobAsync(Arg.Any<CancellationToken>()).Returns(
            _ => Task.FromException<KeyRotationJob?>(new InvalidOperationException("database down")),
            _ =>
            {
                secondCheck.TrySetResult();
                return Task.FromResult<KeyRotationJob?>(null);
            });
        using RotationWorker sut = CreateWorker(failureRetryDelay: TimeSpan.Zero);

        // Act
        await sut.StartAsync(TestContext.Current.CancellationToken);

        // Assert
        Task finished = await Task.WhenAny(secondCheck.Task, sut.ExecuteTask.ShouldNotBeNull());
        finished.ShouldBe(secondCheck.Task);
    }
}
