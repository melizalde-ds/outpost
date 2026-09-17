import grpc
from weatherstream.auth.v1 import auth_pb2, auth_pb2_grpc


def run():
    with grpc.insecure_channel("localhost:50051") as channel:
        stub = auth_pb2_grpc.AuthServiceStub(channel)
        response = stub.Login(auth_pb2.LoginRequest(username="user", password="pass"))
        print("Login response:", response)
