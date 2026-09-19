from weatherstream import get_auth

if __name__ == "__main__":
    username = "admin"
    password = "secret_password"

    client = get_auth()
    response = client.login(username, password)
    print(f"Login response: {response}")
