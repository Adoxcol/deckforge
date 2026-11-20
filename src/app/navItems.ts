import { Gamepad2, Layers, Palette, Puzzle, Sparkles, Usb, Zap } from "lucide-react";

export const NAV_ITEMS = [
    { id: "workspace", label: "Profiles and keys", icon: Layers },
    { id: "macros", label: "Macro studio", icon: Zap },
    { id: "icons", label: "Icon studio", icon: Palette },
    { id: "themes", label: "Themes", icon: Sparkles },
    { id: "gamemode", label: "Game mode", icon: Gamepad2 },
    { id: "plugins", label: "Plugins", icon: Puzzle },
    { id: "devices", label: "Devices", icon: Usb },
  ] as const;
