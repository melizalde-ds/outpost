import os

from google.protobuf import json_format
from auth import get_auth

if __name__ == "__main__":
    username = os.environ["OUTPOST_USERNAME"]
    password = os.environ["OUTPOST_PASSWORD"]

    client = get_auth()

    response = client.login(username, password)

    print("Login response:")
    print(
        json_format.MessageToJson(response, always_print_fields_with_no_presence=True)
    )
