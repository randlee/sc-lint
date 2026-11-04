"""Consumer-wheel content invariants shared by release verification paths."""
from pathlib import PurePosixPath

REPO_ONLY_TESTS = frozenset({
    "test_consumer_contract", "test_consumer_lifecycle",
    "test_orchestration_contracts", "test_release_artifacts",
})


def validate_consumer_wheel(names: list[str]) -> None:
    forbidden = []
    for name in names:
        path = PurePosixPath(name)
        if "sc_lint" in path.parts and "tests" in path.parts and path.name.split(".")[0] in REPO_ONLY_TESTS:
            forbidden.append(name)
    if forbidden:
        raise ValueError(f"sc-lint wheel contains repository-only tests: {sorted(forbidden)}")
