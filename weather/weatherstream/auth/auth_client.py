from typing import Literal

from pydantic import BaseModel
from pydantic.networks import IPvAnyAddress


class AuthConfiguration(BaseModel):
    host: Literal["localhost"] | IPvAnyAddress
    port: int = 50051


class AuthClient:
    def __init__(self, config: AuthConfiguration):
        pass


_Auth: AuthClient | None = None


def get_auth() -> AuthClient:
    raise RuntimeError("AuthClient is not initialized. Call `init_auth` first.")
