"""Platform contracts for tests of Linux/Unix workflow shell snippets."""
import os

import pytest


@pytest.fixture
def posix_shell():
    if os.name == 'nt':
        pytest.skip('POSIX shell fixture uses executable shebang stubs; exercised by the Linux/macOS gates, not the Windows WSL stub')
