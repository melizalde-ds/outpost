using Google.Protobuf;
using Google.Protobuf.Reflection;

using Outpost.Key.V1;

using Shouldly;

namespace KeyProvider.UnitTests.Api;

public sealed class ProtoRedactionTests
{
    private const string Secret = "S3NT1NEL";

    private static readonly Dictionary<string, Type> GeneratedMessages = typeof(SignTokenResponse).Assembly
        .GetTypes()
        .Where(type => type is { IsClass: true, IsAbstract: false } && typeof(IMessage).IsAssignableFrom(type))
        .ToDictionary(type => ((IMessage)Activator.CreateInstance(type)!).Descriptor.FullName);

    public static TheoryData<string, string> DebugRedactFields()
    {
        TheoryData<string, string> data = new();
        foreach ((string messageName, Type type) in GeneratedMessages)
        {
            MessageDescriptor descriptor = ((IMessage)Activator.CreateInstance(type)!).Descriptor;
            foreach (FieldDescriptor field in descriptor.Fields.InFieldNumberOrder())
            {
                if (field.GetOptions()?.DebugRedact == true)
                {
                    data.Add(messageName, field.Name);
                }
            }
        }

        return data;
    }

    private static IMessage WithSecret(string messageName, string fieldName)
    {
        IMessage message = (IMessage)Activator.CreateInstance(GeneratedMessages[messageName])!;
        FieldDescriptor field = message.Descriptor.FindFieldByName(fieldName);
        object value = field.FieldType switch
        {
            FieldType.String => Secret,
            FieldType.Bytes => ByteString.CopyFromUtf8(Secret),
            _ => throw new NotSupportedException($"{messageName}.{fieldName}: add a secret value for {field.FieldType}")
        };
        field.Accessor.SetValue(message, value);
        return message;
    }

    [Theory]
    [MemberData(nameof(DebugRedactFields))]
    public void ToString_DebugRedactField_HidesTheValue(string messageName, string fieldName)
    {
        // Arrange
        IMessage message = WithSecret(messageName, fieldName);

        // Act
        string output = message.ToString()!;

        // Assert
        output.ShouldNotContain(Secret);
    }

    [Fact]
    public void SignTokenResponse_ToString_StillShowsTheKeyId()
    {
        // Arrange
        SignTokenResponse response = new() { Token = Secret, KeyId = "key-id" };

        // Act
        string output = response.ToString();

        // Assert
        output.ShouldContain("key-id");
    }
}
