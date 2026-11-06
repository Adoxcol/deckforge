import React from "react";
import {
  Briefcase,
  Camera,
  CircleDot,
  Clapperboard,
  Cpu,
  Film,
  Folder,
  Gamepad2,
  Globe,
  Layers,
  LayoutGrid,
  Lightbulb,
  MessageSquare,
  Mic,
  MicOff,
  Monitor,
  Music,
  Pause,
  Play,
  Plus,
  Puzzle,
  Radio,
  Scissors,
  Settings,
  Shield,
  Sliders,
  Sparkles,
  Terminal,
  Video,
  Volume2,
  Zap,
} from "lucide-react";

const ICON_MAP: Record<string, React.ComponentType<{ size?: number; className?: string; style?: React.CSSProperties }>> = {
  Radio,
  Clapperboard,
  Film,
  MicOff,
  Mic,
  CircleDot,
  Scissors,
  Camera,
  MessageSquare,
  Monitor,
  Video,
  Sliders,
  Music,
  Lightbulb,
  LayoutGrid,
  Gamepad2,
  Briefcase,
  Layers,
  Cpu,
  Volume2,
  Terminal,
  Zap,
  Globe,
  Folder,
  Sparkles,
  Shield,
  Pause,
  Play,
  Settings,
  Puzzle,
  Plus,
};

export const AVAILABLE_ICON_NAMES = Object.keys(ICON_MAP);

interface DeckIconProps {
  name: string;
  size?: number;
  className?: string;
  color?: string;
}

export const DeckIcon: React.FC<DeckIconProps> = ({ name, size = 20, className = "", color }) => {
  const Comp = ICON_MAP[name] || LayoutGrid;
  return <Comp size={size} className={className} style={color ? { color } : undefined} />;
};
