import json
import os
import subprocess
import tempfile
import urllib.request
from itertools import zip_longest

APP_VERSION = "1.0.10"
UPDATE_URL = "https://raw.githubusercontent.com/Vacuoss/WinFlow/main/updates.json"
GITHUB_URL = "https://github.com/Vacuoss"


def _version_tuple(value: str) -> tuple[int, ...]:
    result: list[int] = []

    for part in value.strip().split("."):
        number = ""

        for char in part:
            if char.isdigit():
                number += char
            else:
                break

        result.append(int(number or 0))

    return tuple(result)


def _is_newer(latest: str, current: str) -> bool:
    latest_parts = _version_tuple(latest)
    current_parts = _version_tuple(current)

    for new, old in zip_longest(latest_parts, current_parts, fillvalue=0):
        if new > old:
            return True

        if new < old:
            return False

    return False


def check_for_updates() -> tuple[str | None, str | None]:
    try:
        with urllib.request.urlopen(UPDATE_URL, timeout=6) as response:
            data = json.loads(response.read().decode("utf-8"))

        latest = str(data.get("version", "")).strip()
        installer_url = str(data.get("installer_url", "")).strip()

        if latest and installer_url and _is_newer(latest, APP_VERSION):
            return latest, installer_url

    except Exception:
        pass

    return None, None


def download_and_run_update(installer_url: str) -> None:
    temp_dir = tempfile.gettempdir()
    installer_path = os.path.join(temp_dir, "WinFlowUpdate.exe")

    urllib.request.urlretrieve(installer_url, installer_path)

    subprocess.Popen([
        installer_path,
        "/VERYSILENT",
        "/NORESTART",
        "/CLOSEAPPLICATIONS",
        "/RESTARTAPPLICATIONS",
    ])

    os._exit(0)
