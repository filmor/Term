from collections.abc import Callable
from typing import Any, Protocol, TypeAlias, TypedDict


class BaseTerm:
    """Base class for all Erlang terms."""


class BasePid(BaseTerm):
    """Serves as a base for local pid and remote pid."""


class BaseRef(BaseTerm):
    """Serves as a base for local ref and remote ref."""


# Type AnyTerm includes Erlang type wrappers + compatible Python types
Term: TypeAlias = (
    str
    | list[Any]
    | tuple[Any, ...]
    | dict[Any, Any]
    | int
    | float
    | bytes
    | BaseTerm
    | None
)

Hook: TypeAlias = Callable[[Any], Any]
Hooks: TypeAlias = dict[str, Hook]


class DecodeOptions(TypedDict, total=False):
    atom: str
    atom_call: Callable[[str], Any]
    byte_string: str
    decode_hook: Hooks


class EncodeOptions(TypedDict, total=False):
    encode_hook: Hooks | None


class LegacyEncodeOptions(TypedDict):
    encode_hook: Hook


class Node(Protocol):
    @property
    def node_name_(self) -> str: ...
