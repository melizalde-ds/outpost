using Key.Engine;
using Key.Worker;


var build = WebApplication.CreateBuilder(args);

build.Services.AddSingleton<ED25519Engine>();

build.Services.Configure<WorkerOptions>(build.Configuration.GetSection("Worker"));
build.Services.AddHostedService<Worker>();

var app = build.Build();

await app.RunAsync();
