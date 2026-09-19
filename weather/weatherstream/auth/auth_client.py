import atexit
import itertools
import threading
from typing import Literal

import grpc
from grpc import Channel
from pydantic import BaseModel, Field, HttpUrl, IPvAnyAddress

from weatherstream.auth.v1.auth_pb2 import (
    LoginRequest,
    RefreshTokenRequest,
    ValidateTokenRequest,
)
from weatherstream.auth.v1.auth_pb2_grpc import AuthServiceStub


class AuthConfiguration(BaseModel):
    host: Literal["localhost"] | HttpUrl | IPvAnyAddress = "localhost"
    port: int = Field(
        default=50051,
        ge=1,
        le=65535,
        description="The port number for the gRPC server",
    )
    pool_size: int = Field(
        default=3,
        ge=1,
        le=100,
        description="The maximum number of connections in the pool",
    )


class AuthClient:
    def __init__(self, config: AuthConfiguration):
        self._channels = []
        self._stubs = []
        self.pool_size = config.pool_size
        self._counter = itertools.count()
        self._lock = threading.Lock()
        self._closed = False

        for _ in range(config.pool_size):
            channel: Channel = grpc.insecure_channel(
                f"{config.host}:{config.port}",
                options=[("grpc.use_local_subchannel_pool", 1)],
            )
            stub = AuthServiceStub(channel)

            self._channels.append(channel)
            self._stubs.append(stub)

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc_value, traceback):
        self.close()

    def _next(self) -> AuthServiceStub:
        if self._closed:
            raise RuntimeError("AuthClient is closed. Cannot perform operations.")
        with self._lock:
            index = next(self._counter) % self.pool_size
        return self._stubs[index]

    def close(self) -> None:
        if not self._closed:
            for channel in self._channels:
                channel.close()
            self._closed = True

    def login(self, username: str, password: str):
        stub = self._next()
        request = LoginRequest(username=username, password=password)
        result = stub.Login(request)
        return result

    def validate_token(self, token: str):
        stub = self._next()
        request = ValidateTokenRequest(access_token=token)
        result = stub.ValidateToken(request)
        return result

    def refresh_token(self, token: str):
        stub = self._next()
        request = RefreshTokenRequest(refresh_token=token)
        result = stub.RefreshToken(request)
        return result


_auth: AuthClient | None = None
_auth_lock = threading.Lock()


def get_auth() -> AuthClient:
    global _auth
    if _auth is None:
        with _auth_lock:
            if _auth is None:
                _auth = AuthClient(AuthConfiguration())
                atexit.register(_auth.close)
    return _auth


def close_auth() -> None:
    global _auth
    if _auth is not None:
        with _auth_lock:
            if _auth is not None:
                _auth.close()
                atexit.unregister(_auth.close)
                _auth = None
