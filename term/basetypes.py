from typing import Any


class BaseTerm:
    """Base class for all Erlang terms."""


class BasePid(BaseTerm):
    """Serves as a base for local pid and remote pid."""


class BaseRef(BaseTerm):
    """Serves as a base for local ref and remote ref."""


# Type AnyTerm includes Erlang type wrappers + compatible Python types
Term = str | list[Any] | tuple | dict | int | float | bytes | BaseTerm | None
