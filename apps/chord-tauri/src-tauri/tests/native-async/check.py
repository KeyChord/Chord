"""Run after building Chord and the Swift fixture; every child has a hang timeout."""
import os
from pathlib import Path
import subprocess
import sys

chord, addon = (str(Path(arg).resolve()) for arg in sys.argv[1:])
script = str(Path(__file__).with_name("verify.ts"))
for command, top_level, succeeds in [
    (["bun", script], "1", True),
    (["js", script], "1", True),
    (["bun", script], "fail", False),
    (["run", script, "verify"], "0", True),
    (["run", script, "fail"], "0", False),
]:
    result = subprocess.run(
        [chord, *command],
        env={**os.environ, "CHORD_ASYNC_ADDON": addon, "CHORD_ASYNC_TOP_LEVEL": top_level},
        capture_output=True, text=True, timeout=10,
    )
    assert (result.returncode == 0) == succeeds, (command, result.stdout, result.stderr)
    if succeeds:
        assert "native-async-ok" in result.stdout, result.stdout
    else:
        assert "expectedFailure" in result.stderr, result.stderr
    print("PASS", *command[:1], "success" if succeeds else "error propagation")
