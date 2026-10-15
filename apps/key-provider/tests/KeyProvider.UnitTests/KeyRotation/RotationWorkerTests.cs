using KeyProvider.KeyRotation;

using Microsoft.Extensions.Logging.Abstractions;
using Microsoft.Extensions.Options;
using Microsoft.Extensions.Time.Testing;

using NSubstitute;

using Polly;
using Polly.Registry;

using Shouldly;

namespace KeyProvider.UnitTests.KeyRotation;

public sealed class RotationWorkerTests : IDisposable
{
    private static readonly DateTimeOffset Now = new(2026, 1, 1, 0, 0, 0, TimeSpan.Zero);
    private static readonly TimeSpan WaitLimit = TimeSpan.FromSeconds(5);
    private static readonly TimeSpan ClockStep = TimeSpan.FromMinutes(1);

    private readonly IKeyRotationService _rotation = Substitute.For<IKeyRotationService>();
    private readonly FakeTimeProvider _clock = new(Now);
    private readonly TaskCompletionSource _secondCheck = new(TaskCreationOptions.RunContinuationsAsynchronously);
    private readonly TaskCompletionSource _rotated = new(TaskCreationOptions.RunContinuationsAsynchronously);
    private readonly RotationWorker _worker;

    public RotationWorkerTests()
    {
        ResiliencePipelineProvider<string> pipelines = Substitute.For<ResiliencePipelineProvider<string>>();
        pipelines.GetPipeline<KeyRotationResult>(Arg.Any<string>()).Returns(ResiliencePipeline<KeyRotationResult>.Empty);

        _rotation.RotateAsync(Arg.Any<CancellationToken>()).Returns(_ =>
        {
            _rotated.TrySetResult();
            return new KeyRotationSucceeded(Now, Now, Guid.Empty);
        });

        _worker = new RotationWorker(
            _rotation,
            pipelines,
            Options.Create(new RotationScheduleOptions { Expression = "0 */8 * * *" }),
            NullLogger<RotationWorker>.Instance,
            _clock);
    }

    public void Dispose()
    {
        _worker.Dispose();
    }

    private void LastRotationFinishedAt(DateTimeOffset? finishedAt)
    {
        KeyRotationJob? job = finishedAt is { } at
            ? new KeyRotationJob { KeyPairRotationJobId = Guid.Empty, StartedAt = at, FinishedAt = at, Success = true }
            : null;

        int checks = 0;
        _rotation.GetLatestSuccessfulJobAsync(Arg.Any<CancellationToken>()).Returns(_ =>
        {
            if (Interlocked.Increment(ref checks) == 2)
            {
                _secondCheck.TrySetResult();
            }

            return job;
        });
    }

    private async Task RunUntilTheSecondCheckAsync()
    {
        CancellationToken ct = TestContext.Current.CancellationToken;
        await _worker.StartAsync(ct);

        using CancellationTokenSource limit = CancellationTokenSource.CreateLinkedTokenSource(ct);
        limit.CancelAfter(WaitLimit);
        while (!_secondCheck.Task.IsCompleted)
        {
            _clock.Advance(ClockStep);
            await Task.Delay(TimeSpan.FromMilliseconds(20), limit.Token);
        }

        await _worker.StopAsync(ct);
    }

    [Fact]
    public async Task StartAsync_NoRotationYet_RotatesRightAway()
    {
        // Arrange
        LastRotationFinishedAt(null);

        // Act
        await _worker.StartAsync(TestContext.Current.CancellationToken);

        // Assert
        await _rotated.Task.WaitAsync(WaitLimit, TestContext.Current.CancellationToken);
    }

    [Fact]
    public async Task StartAsync_LastRotationBeforeTheCurrentSlot_RotatesRightAway()
    {
        // Arrange
        LastRotationFinishedAt(Now.AddMinutes(-1));

        // Act
        await _worker.StartAsync(TestContext.Current.CancellationToken);

        // Assert
        await _rotated.Task.WaitAsync(WaitLimit, TestContext.Current.CancellationToken);
    }

    [Fact]
    public async Task StartAsync_LastRotationInTheCurrentSlot_DoesNotRotate()
    {
        // Arrange
        LastRotationFinishedAt(Now);

        // Act
        await RunUntilTheSecondCheckAsync();

        // Assert
        await _rotation.DidNotReceive().RotateAsync(Arg.Any<CancellationToken>());
    }
}
