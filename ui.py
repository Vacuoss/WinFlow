import json
import os
import sys
import socket
import subprocess
import threading
import webbrowser
import customtkinter as ctk
from PIL import Image

from update_manager import (
    APP_VERSION,
    GITHUB_URL,
    check_for_updates,
    download_and_run_update,
)

APPDATA_DIR = os.path.join(os.getenv("APPDATA", "."), "WinFlow")
os.makedirs(APPDATA_DIR, exist_ok=True)

CONFIG_PATH = os.path.join(APPDATA_DIR, "config.json")

MAIN = "#484a77"
MAIN_DARK = "#35375d"
MAIN_LIGHT = "#62649a"

GREEN = "#1ebc73"
GREEN_DARK = "#159b5c"

RED = "#ae2334"
RED_DARK = "#8d1b2a"

BG = "#161724"
CARD = "#202136"
PANEL = "#292a44"
INPUT_BG = "#1b1c2d"
TEXT_MUTED = "#b9bad7"
TEXT_DIM = "#9294bd"

DEFAULT_CONFIG = {
    "device_id": "",
    "device_name": os.environ.get("COMPUTERNAME", "Windows-PC"),
    "tcp_port": 45455,
    "udp_port": 45456,
    "peer_ip": "",
    "peer_tcp_port": 45455,
    "peer_udp_port": 45456,
    "switch_edge": "right",
    "clipboard_enabled": True,
    "hotkey_switch": "Ctrl+Alt+Right",
    "hotkey_disconnect": "Shift+LeftAlt",
    "autostart_enabled": False
}


VK_NAMES = {
    0x08: "Backspace",
    0x09: "Tab",
    0x0D: "Enter",
    0x10: "Shift",
    0x11: "Ctrl",
    0x12: "Alt",
    0x1B: "Esc",
    0x20: "Space",
    0x25: "Left",
    0x26: "Up",
    0x27: "Right",
    0x28: "Down",
    0x2E: "Delete",
    0xA4: "LeftAlt",
    0xA5: "RightAlt",
}

for code in range(0x30, 0x3A):
    VK_NAMES[code] = chr(code)

for code in range(0x41, 0x5B):
    VK_NAMES[code] = chr(code)

for i in range(1, 25):
    VK_NAMES[0x6F + i] = f"F{i}"


def resource_path(relative_path: str) -> str:
    if hasattr(sys, "_MEIPASS"):
        return os.path.join(sys._MEIPASS, relative_path)

    candidates = [
        os.path.join(os.path.abspath("."), relative_path),
        os.path.join(os.path.abspath("."), "target", "release", relative_path),
        os.path.join(os.path.dirname(os.path.abspath(__file__)), relative_path),
    ]

    for candidate in candidates:
        if os.path.exists(candidate):
            return candidate

    return candidates[0]


EXE_PATH = resource_path("winflow-core.exe")


def get_local_ip() -> str:
    try:
        sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        sock.connect(("8.8.8.8", 80))
        ip = sock.getsockname()[0]
        sock.close()

        if ip and not ip.startswith("127."):
            return ip
    except Exception:
        pass

    try:
        host = socket.gethostname()
        for ip in socket.gethostbyname_ex(host)[2]:
            if ip and not ip.startswith("127."):
                return ip
    except Exception:
        pass

    return "127.0.0.1"


def load_config() -> dict:
    if not os.path.exists(CONFIG_PATH):
        save_config(DEFAULT_CONFIG)
        return DEFAULT_CONFIG.copy()

    try:
        with open(CONFIG_PATH, "r", encoding="utf-8") as file:
            loaded = json.load(file)
    except Exception:
        loaded = {}

    cfg = DEFAULT_CONFIG.copy()
    cfg.update(loaded)
    return cfg


def save_config(cfg: dict) -> None:
    with open(CONFIG_PATH, "w", encoding="utf-8") as file:
        json.dump(cfg, file, indent=4, ensure_ascii=False)


def set_windows_autostart(enabled: bool) -> None:
    if os.name != "nt":
        return

    try:
        import winreg

        app_name = "WinFlow"
        exe_path = sys.executable if getattr(sys, "frozen", False) else os.path.abspath(__file__)

        key = winreg.OpenKey(
            winreg.HKEY_CURRENT_USER,
            r"Software\Microsoft\Windows\CurrentVersion\Run",
            0,
            winreg.KEY_SET_VALUE,
        )

        if enabled:
            winreg.SetValueEx(key, app_name, 0, winreg.REG_SZ, f'"{exe_path}"')
        else:
            try:
                winreg.DeleteValue(key, app_name)
            except FileNotFoundError:
                pass

        winreg.CloseKey(key)
    except Exception:
        pass


class HotkeyField(ctk.CTkFrame):
    def __init__(self, parent, title: str, variable: ctk.StringVar, root, on_change=None):
        super().__init__(parent, fg_color="transparent")

        self.variable = variable
        self.root = root
        self.on_change = on_change
        self.capture = False
        self.keys: list[str] = []

        ctk.CTkLabel(
            self,
            text=title,
            font=("Segoe UI", 11, "bold"),
            text_color="#d9daf0",
            anchor="w"
        ).pack(fill="x", pady=(5, 3))

        self.button = ctk.CTkButton(
            self,
            text=self.variable.get(),
            height=33,
            corner_radius=10,
            fg_color=INPUT_BG,
            hover_color=MAIN_DARK,
            border_width=1,
            border_color=MAIN_LIGHT,
            font=("Segoe UI", 11),
            command=self.start_capture
        )
        self.button.pack(fill="x")

    def start_capture(self):
        self.capture = True
        self.keys = []
        self.button.configure(text="Press shortcut...", fg_color=MAIN_DARK)
        self.root.bind("<KeyPress>", self.capture_key)
        self.root.bind("<KeyRelease>", self.finish_capture)

    def capture_key(self, event):
        if not self.capture:
            return "break"
        key = VK_NAMES.get(event.keycode)

        if not key:
            return "break"

        if key not in self.keys:
            self.keys.append(key)

        value = "+".join(self.order_keys(self.keys))
        self.variable.set(value)
        self.button.configure(text=value)

        return "break"

    def finish_capture(self, _event=None):
        if not self.capture:
            return

        if not self.keys:
            return

        self.capture = False
        self.button.configure(fg_color=INPUT_BG)
        self.root.unbind("<KeyPress>")
        self.root.unbind("<KeyRelease>")

        if self.on_change:
            self.on_change()

    @staticmethod
    def order_keys(keys: list[str]) -> list[str]:
        priority = ["Ctrl", "Shift", "Alt", "LeftAlt", "RightAlt"]
        result = []

        for item in priority:
            if item in keys and item not in result:
                result.append(item)

        for item in keys:
            if item not in result:
                result.append(item)

        return result

class SecretField(ctk.CTkFrame):
    def __init__(
        self,
        parent,
        title: str,
        variable: ctk.StringVar,
        eye_open,
        eye_closed,
        placeholder: str = "",
        editable: bool = True,
        on_change=None,
        hidden: bool = False,
        height: int = 32,
    ):
        super().__init__(parent, fg_color="transparent")

        self.variable = variable
        self.on_change = on_change
        self.eye_open = eye_open
        self.eye_closed = eye_closed
        self.visible = not hidden
        self.label = ctk.CTkLabel(
            self,
            text=title,
            font=("Segoe UI", 11, "bold"),
            text_color="#d9daf0",
            anchor="w"
        )
        self.label.pack(fill="x", pady=(5, 2))
        self.row = ctk.CTkFrame(self, fg_color="transparent")
        self.row.pack(fill="x")
        self.entry = ctk.CTkEntry(
            self.row,
            textvariable=self.variable,
            placeholder_text=placeholder,
            height=height,
            corner_radius=10,
            fg_color=INPUT_BG,
            border_color=MAIN_LIGHT,
            state="normal" if editable else "disabled",
            font=("Segoe UI", 12)
        )
        self.entry.pack(side="left", fill="x", expand=True)

        self.eye_btn = ctk.CTkButton(
            self.row,
            text="",
            image=self.eye_open if self.visible else self.eye_closed,
            width=38,
            height=height,
            corner_radius=10,
            fg_color=MAIN_DARK,
            hover_color=MAIN_LIGHT,
            command=self.toggle
        )
        self.eye_btn.pack(side="right", padx=(7, 0))

        if editable:
            self.entry.bind("<KeyRelease>", self._capture_value)
            self.entry.bind("<FocusOut>", self._capture_value)

        self.apply_visibility()

    def _capture_value(self, _event=None):
        if self.on_change:
            self.on_change()

    def get(self) -> str:
        return self.variable.get()

    def set(self, value: str):
        self.variable.set(value)
        self.apply_visibility()

    def toggle(self):
        self.visible = not self.visible
        self.apply_visibility()

    def apply_visibility(self):
        if self.visible:
            self.entry.configure(show="")
            self.eye_btn.configure(image=self.eye_open)
        else:
            self.entry.configure(show="•")
            self.eye_btn.configure(image=self.eye_closed)


class WinFlowUI:
    def __init__(self):
        ctk.set_appearance_mode("dark")
        ctk.set_default_color_theme("blue")
        self.root = ctk.CTk()
        self.root.title("WinFlow")
        self.root.geometry("500x610")
        self.root.resizable(False, False)
        self.root.protocol("WM_DELETE_WINDOW", self.on_close)
        self.cfg = load_config()
        self.process = None
        self.connected = False
        self.settings_window = None
        self.update_window = None
        self.device_name = ctk.StringVar(value=self.cfg.get("device_name", "Windows-PC"))
        self.local_ip = ctk.StringVar(value=get_local_ip())
        self.peer_ip = ctk.StringVar(value=self.cfg.get("peer_ip", ""))
        self.edge = ctk.StringVar(value=self.cfg.get("switch_edge", "right"))
        self.clipboard = ctk.BooleanVar(value=self.cfg.get("clipboard_enabled", True))
        self.hotkey_switch = ctk.StringVar(value=self.cfg.get("hotkey_switch", "Ctrl+Alt+Right"))
        self.hotkey_disconnect = ctk.StringVar(value=self.cfg.get("hotkey_disconnect", "Shift+LeftAlt"))
        self.autostart_enabled = ctk.BooleanVar(value=self.cfg.get("autostart_enabled", False))
        self.status = ctk.StringVar(value="Disconnected")
        self.agent_log = ctk.StringVar(value="Agent is not running")
        self.eye_open_img = ctk.CTkImage(
            light_image=Image.open(resource_path("eye.png")),
            dark_image=Image.open(resource_path("eye.png")),
            size=(15, 15)
        )

        self.eye_closed_img = ctk.CTkImage(
            light_image=Image.open(resource_path("eye_off.png")),
            dark_image=Image.open(resource_path("eye_off.png")),
            size=(15, 15)
        )

        self.build()
        self.root.after(1600, self.auto_check_updates)

    def build(self):
        self.root.configure(fg_color=BG)

        container = ctk.CTkFrame(self.root, corner_radius=20, fg_color=CARD)
        container.pack(padx=16, pady=16, fill="both", expand=True)

        header = ctk.CTkFrame(container, fg_color="transparent", height=42)
        header.pack(padx=18, pady=(12, 2), fill="x")
        header.pack_propagate(False)

        spacer_left = ctk.CTkFrame(header, fg_color="transparent", width=44)
        spacer_left.pack(side="left")

        ctk.CTkLabel(
            header,
            text="WinFlow",
            font=("Segoe UI", 28, "bold"),
            text_color="white"
        ).pack(side="left", fill="x", expand=True)

        ctk.CTkButton(
            header,
            text="⚙",
            width=44,
            height=34,
            corner_radius=12,
            fg_color=MAIN_DARK,
            hover_color=MAIN_LIGHT,
            font=("Segoe UI Emoji", 18),
            command=self.open_settings
        ).pack(side="right")

        ctk.CTkLabel(
            container,
            text="Cursor • Clipboard",
            font=("Segoe UI", 12),
            text_color=TEXT_MUTED
        ).pack(pady=(0, 6))

        self.status_badge = ctk.CTkLabel(
            container,
            textvariable=self.status,
            width=156,
            height=30,
            corner_radius=15,
            fg_color=RED,
            text_color="white",
            font=("Segoe UI", 12, "bold")
        )
        self.status_badge.pack(pady=(0, 7))

        form = ctk.CTkFrame(container, fg_color=PANEL, corner_radius=15)
        form.pack(padx=16, pady=2, fill="x")

        self.device_field = SecretField(
            form,
            title="Device name",
            eye_open=self.eye_open_img,
            eye_closed=self.eye_closed_img,
            variable=self.device_name,
            placeholder="Device name",
            editable=True,
            on_change=self.save_silent,
            hidden=True,
            height=32,
        )
        self.device_field.pack(padx=13, fill="x")

        self.local_ip_field = SecretField(
            form,
            title="This device IP",
            eye_open=self.eye_open_img,
            eye_closed=self.eye_closed_img,
            variable=self.local_ip,
            placeholder="Local IP",
            editable=False,
            hidden=False,
            height=32,
        )
        self.local_ip_field.pack(padx=13, fill="x")

        ip_actions = ctk.CTkFrame(form, fg_color="transparent")
        ip_actions.pack(padx=13, pady=(4, 0), fill="x")

        ctk.CTkButton(
            ip_actions,
            text="Refresh",
            width=86,
            height=28,
            corner_radius=9,
            fg_color=MAIN,
            hover_color=MAIN_LIGHT,
            font=("Segoe UI", 11),
            command=self.refresh_ip
        ).pack(side="left")

        ctk.CTkButton(
            ip_actions,
            text="Copy",
            width=70,
            height=28,
            corner_radius=9,
            fg_color=MAIN_DARK,
            hover_color=MAIN_LIGHT,
            font=("Segoe UI", 11),
            command=self.copy_ip
        ).pack(side="left", padx=(7, 0))

        self.peer_field = SecretField(
            form,
            title="Peer IP",
            eye_open=self.eye_open_img,
            eye_closed=self.eye_closed_img,
            variable=self.peer_ip,
            placeholder="Example: 192.168.1.25",
            editable=True,
            on_change=self.save_silent,
            hidden=False,
            height=32,
        )
        self.peer_field.pack(padx=13, fill="x")

        row = ctk.CTkFrame(form, fg_color="transparent")
        row.pack(padx=13, pady=(7, 7), fill="x")

        ctk.CTkLabel(
            row,
            text="Switch edge",
            font=("Segoe UI", 11, "bold"),
            text_color="#d9daf0",
            width=86,
            anchor="w"
        ).pack(side="left")

        self.edge_menu = ctk.CTkOptionMenu(
            row,
            values=["right", "left", "top", "bottom"],
            variable=self.edge,
            width=124,
            height=31,
            corner_radius=9,
            fg_color=MAIN,
            button_color=MAIN_DARK,
            button_hover_color=MAIN_LIGHT,
            font=("Segoe UI", 11),
            command=lambda _: self.save_silent()
        )
        self.edge_menu.pack(side="left", padx=(7, 0))

        checks = ctk.CTkFrame(form, fg_color="transparent")
        checks.pack(padx=13, pady=(0, 10), fill="x")

        self.clipboard_check = ctk.CTkCheckBox(
            checks,
            text="Clipboard",
            variable=self.clipboard,
            fg_color=GREEN,
            hover_color=GREEN_DARK,
            checkmark_color="white",
            font=("Segoe UI", 11),
            command=self.save_silent
        )
        self.clipboard_check.pack(side="left")

        self.main_btn = ctk.CTkButton(
            container,
            text="Connect",
            height=42,
            corner_radius=14,
            fg_color=GREEN,
            hover_color=GREEN_DARK,
            font=("Segoe UI", 14, "bold"),
            command=self.toggle_connection
        )
        self.main_btn.pack(fill="x", padx=16, pady=(12, 6))

        self.log_label = ctk.CTkLabel(
            container,
            textvariable=self.agent_log,
            font=("Segoe UI", 10),
            text_color=TEXT_DIM,
            wraplength=415
        )
        self.log_label.pack(pady=(0, 5))

        footer = ctk.CTkFrame(container, fg_color="transparent")
        footer.pack(pady=(0, 5))

        ctk.CTkLabel(
            footer,
            text=f"WinFlow {APP_VERSION} • Made by Avacuoss",
            font=("Segoe UI", 10, "bold"),
            text_color=TEXT_DIM
        ).pack(side="left")

        ctk.CTkButton(
            footer,
            text="GitHub",
            width=68,
            height=25,
            corner_radius=9,
            fg_color=MAIN_DARK,
            hover_color=MAIN_LIGHT,
            font=("Segoe UI", 10),
            command=lambda: webbrowser.open(GITHUB_URL)
        ).pack(side="left", padx=(7, 0))

    def open_settings(self):
        if self.settings_window is not None and self.settings_window.winfo_exists():
            self.settings_window.focus()
            return

        self.settings_window = ctk.CTkToplevel(self.root)
        self.settings_window.title("WinFlow Settings")
        self.settings_window.geometry("420x400")
        self.settings_window.resizable(False, False)
        self.settings_window.configure(fg_color=BG)
        self.settings_window.transient(self.root)
        self.settings_window.grab_set()

        panel = ctk.CTkFrame(self.settings_window, fg_color=CARD, corner_radius=18)
        panel.pack(padx=14, pady=14, fill="both", expand=True)

        ctk.CTkLabel(
            panel,
            text="Settings",
            font=("Segoe UI", 22, "bold"),
            text_color="white"
        ).pack(pady=(14, 8))

        HotkeyField(
            panel,
            "Switch hotkey",
            self.hotkey_switch,
            self.settings_window,
            on_change=self.save_settings
        ).pack(padx=20, fill="x")

        HotkeyField(
            panel,
            "Disconnect hotkey",
            self.hotkey_disconnect,
            self.settings_window,
            on_change=self.save_settings
        ).pack(padx=20, fill="x")

        ctk.CTkCheckBox(
            panel,
            text="Start WinFlow with Windows",
            variable=self.autostart_enabled,
            fg_color=GREEN,
            hover_color=GREEN_DARK,
            checkmark_color="white",
            font=("Segoe UI", 11),
            command=self.save_settings
        ).pack(padx=20, pady=(12, 8), anchor="w")

        ctk.CTkButton(
            panel,
            text="Save settings",
            height=36,
            corner_radius=12,
            fg_color=GREEN,
            hover_color=GREEN_DARK,
            font=("Segoe UI", 12, "bold"),
            command=self.save_settings
        ).pack(padx=20, pady=(8, 8), fill="x")

        ctk.CTkButton(
            panel,
            text="Check for updates",
            height=34,
            corner_radius=12,
            fg_color=MAIN_DARK,
            hover_color=MAIN_LIGHT,
            font=("Segoe UI", 11, "bold"),
            command=self.check_updates_ui
        ).pack(padx=20, pady=(0, 8), fill="x")

    def refresh_ip(self):
        new_ip = get_local_ip()
        self.local_ip_field.set(new_ip)
        self.agent_log.set("Local IP refreshed")

    def copy_ip(self):
        self.root.clipboard_clear()
        self.root.clipboard_append(self.local_ip_field.get())
        self.agent_log.set("Local IP copied")

    def save_silent(self):
        self.cfg["device_name"] = self.device_field.get().strip() or "Windows-PC"
        self.cfg["peer_ip"] = self.peer_field.get().strip()
        self.cfg["switch_edge"] = self.edge.get()
        self.cfg["clipboard_enabled"] = self.clipboard.get()
        self.cfg["hotkey_switch"] = self.hotkey_switch.get().strip() or "Ctrl+Alt+Right"
        self.cfg["hotkey_disconnect"] = self.hotkey_disconnect.get().strip() or "Shift+LeftAlt"
        self.cfg["autostart_enabled"] = self.autostart_enabled.get()
        save_config(self.cfg)

    def save_settings(self):
        self.save_silent()
        set_windows_autostart(self.autostart_enabled.get())
        self.agent_log.set("Settings saved")

    def toggle_connection(self):
        if self.connected or (self.process is not None and self.process.poll() is None):
            self.disconnect()
        else:
            self.connect()

    def connect(self):
        self.save_silent()

        ip = self.peer_field.get().strip()
        local_ip = self.local_ip_field.get().strip()

        if not ip:
            self.show_error_window("WinFlow", "Enter the IP address of the second device.")
            return

        if ip == local_ip:
            self.show_error_window(
                "WinFlow",
                "Peer IP matches this device IP. Please enter the IP address of the second PC."
            )
            return

        if not os.path.exists(EXE_PATH):
            self.show_error_window(
                "WinFlow",
                "winflow-core.exe was not found.\n\nBuild the Rust core first."
            )
            return

        self.set_status("Connecting", MAIN)
        self.agent_log.set(f"Starting core: {EXE_PATH}")

        creationflags = 0
        if os.name == "nt":
            creationflags = subprocess.CREATE_NO_WINDOW

        self.process = subprocess.Popen(
            [EXE_PATH, "--connect", ip],
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            encoding="utf-8",
            errors="replace",
            creationflags=creationflags
        )

        threading.Thread(target=self.read_agent_output, daemon=True).start()

    def show_error_window(self, title: str, message: str):
        dialog = ctk.CTkToplevel(self.root)
        dialog.title(title)
        dialog.geometry("380x190")
        dialog.resizable(False, False)
        dialog.configure(fg_color=BG)
        dialog.transient(self.root)
        dialog.grab_set()

        panel = ctk.CTkFrame(dialog, fg_color=CARD, corner_radius=18)
        panel.pack(padx=16, pady=16, fill="both", expand=True)

        ctk.CTkLabel(
            panel,
            text=title,
            font=("Segoe UI", 18, "bold"),
            text_color="white"
        ).pack(pady=(16, 8))

        ctk.CTkLabel(
            panel,
            text=message,
            font=("Segoe UI", 12),
            text_color=TEXT_MUTED,
            wraplength=310
        ).pack(pady=(0, 16))

        ctk.CTkButton(
            panel,
            text="OK",
            width=110,
            height=34,
            corner_radius=12,
            fg_color=MAIN,
            hover_color=MAIN_LIGHT,
            command=dialog.destroy
        ).pack()

    def read_agent_output(self):
        if self.process is None or self.process.stdout is None:
            return

        for line in self.process.stdout:
            line = line.strip()

            if not line:
                continue

            self.root.after(0, lambda value=line: self.agent_log.set(value))

            if "WINFLOW_CONNECTED" in line:
                self.root.after(0, lambda: self.set_status("Connected", GREEN))
                self.root.after(0, self.set_connected_button)

            elif "WINFLOW_DISCONNECTED" in line:
                self.root.after(0, lambda: self.set_status("Disconnected", RED))
                self.root.after(0, self.set_disconnected_button)

            elif "WINFLOW_STARTING" in line:
                self.root.after(0, lambda: self.set_status("Connecting", MAIN))

        self.root.after(0, lambda: self.set_status("Disconnected", RED))
        self.root.after(0, self.set_disconnected_button)
        self.root.after(0, lambda: self.agent_log.set("Agent stopped"))

    def disconnect(self):
        if self.process is not None and self.process.poll() is None:
            self.process.terminate()

            try:
                self.process.wait(timeout=2)
            except subprocess.TimeoutExpired:
                self.process.kill()

        self.process = None
        self.set_status("Disconnected", RED)
        self.set_disconnected_button()
        self.agent_log.set("Agent stopped")

    def set_status(self, text: str, color: str):
        self.status.set(text)
        self.status_badge.configure(fg_color=color)

    def set_connected_button(self):
        self.connected = True
        self.main_btn.configure(text="Disconnect", fg_color=RED, hover_color=RED_DARK)

    def set_disconnected_button(self):
        self.connected = False
        self.main_btn.configure(text="Connect", fg_color=GREEN, hover_color=GREEN_DARK)

    def auto_check_updates(self):
        latest, installer_url = check_for_updates()

        if latest and installer_url:
            self.show_update_window(latest, installer_url)

    def check_updates_ui(self):
        self.agent_log.set("Checking for updates...")

        latest, installer_url = check_for_updates()

        if latest and installer_url:
            self.show_update_window(latest, installer_url)
            self.agent_log.set(f"Update available: {latest}")
        else:
            self.show_no_updates_window()
            self.agent_log.set("No updates available")

    def show_no_updates_window(self):
        dialog = ctk.CTkToplevel(self.root)
        dialog.title("WinFlow Update")
        dialog.geometry("340x170")
        dialog.resizable(False, False)
        dialog.configure(fg_color=BG)
        dialog.transient(self.root)
        dialog.grab_set()

        panel = ctk.CTkFrame(dialog, fg_color=CARD, corner_radius=18)
        panel.pack(padx=16, pady=16, fill="both", expand=True)

        ctk.CTkLabel(
            panel,
            text="No updates available",
            font=("Segoe UI", 18, "bold"),
            text_color="white"
        ).pack(pady=(18, 8))

        ctk.CTkLabel(
            panel,
            text=f"WinFlow {APP_VERSION} is up to date.",
            font=("Segoe UI", 12),
            text_color=TEXT_MUTED
        ).pack(pady=(0, 16))

        ctk.CTkButton(
            panel,
            text="OK",
            width=110,
            height=34,
            corner_radius=12,
            fg_color=MAIN,
            hover_color=MAIN_LIGHT,
            command=dialog.destroy
        ).pack()

    def show_update_window(self, latest: str, installer_url: str):
        if self.update_window is not None and self.update_window.winfo_exists():
            self.update_window.focus()
            return

        self.update_window = ctk.CTkToplevel(self.root)
        self.update_window.title("WinFlow Update")
        self.update_window.geometry("380x210")
        self.update_window.resizable(False, False)
        self.update_window.configure(fg_color=BG)
        self.update_window.transient(self.root)
        self.update_window.grab_set()

        panel = ctk.CTkFrame(self.update_window, fg_color=CARD, corner_radius=18)
        panel.pack(padx=16, pady=16, fill="both", expand=True)

        ctk.CTkLabel(
            panel,
            text="Update available",
            font=("Segoe UI", 20, "bold"),
            text_color="white"
        ).pack(pady=(18, 6))

        ctk.CTkLabel(
            panel,
            text=f"WinFlow {latest} is available.",
            font=("Segoe UI", 12),
            text_color=TEXT_MUTED
        ).pack(pady=(0, 18))

        buttons = ctk.CTkFrame(panel, fg_color="transparent")
        buttons.pack(fill="x", padx=18)

        ctk.CTkButton(
            buttons,
            text="Remind me later",
            height=36,
            corner_radius=12,
            fg_color=MAIN_DARK,
            hover_color=MAIN_LIGHT,
            command=self.update_window.destroy
        ).pack(side="left", fill="x", expand=True, padx=(0, 6))

        ctk.CTkButton(
            buttons,
            text="Update",
            height=36,
            corner_radius=12,
            fg_color=GREEN,
            hover_color=GREEN_DARK,
            command=lambda: self.start_update(installer_url)
        ).pack(side="left", fill="x", expand=True, padx=(6, 0))

    def start_update(self, installer_url: str):
        self.agent_log.set("Downloading update...")

        if self.process is not None and self.process.poll() is None:
            self.disconnect()

        threading.Thread(
            target=lambda: download_and_run_update(installer_url),
            daemon=True
        ).start()

    def on_close(self):
        self.disconnect()
        self.root.destroy()

    def run(self):
        self.root.mainloop()


if __name__ == "__main__":
    WinFlowUI().run()