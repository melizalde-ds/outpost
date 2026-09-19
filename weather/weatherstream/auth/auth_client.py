from pydantic import BaseModel
from pydantic.networks import IPvAnyAddress


class AuthConfiguration(BaseModel):
    host: Union[str, IPvAnyAddress]
    port: int = 50051


class AuthClient:
    def __init__(self, config: AuthConfiguration):
        pass
