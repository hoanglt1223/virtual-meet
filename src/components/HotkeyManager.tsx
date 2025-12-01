import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Switch } from "@/components/ui/switch";
import {
  Plus,
  Trash2,
  Edit,
  Play,
  Pause,
  Camera,
  Mic,
  MicOff,
  Volume2,
  Settings as SettingsIcon,
  Power
} from "lucide-react";
import type {
  Hotkey,
  HotkeyAction,
  HotkeyCategory,
  HotkeyRegistrationResponse,
  HotkeyListResponse
} from "@/types";

const ACTION_ICONS = {
  StartVideo: { icon: Play, color: "text-green-600", label: "Start Video" },
  StopVideo: { icon: Pause, color: "text-red-600", label: "Stop Video" },
  StartAudio: { icon: Play, color: "text-green-600", label: "Start Audio" },
  StopAudio: { icon: Pause, color: "text-red-600", label: "Stop Audio" },
  StartRecording: { icon: Camera, color: "text-red-600", label: "Start Recording" },
  StopRecording: { icon: Camera, color: "text-gray-600", label: "Stop Recording" },
  ToggleMute: { icon: MicOff, color: "text-orange-600", label: "Toggle Mute" },
  VolumeUp: { icon: Volume2, color: "text-blue-600", label: "Volume Up" },
  VolumeDown: { icon: Volume2, color: "text-blue-600", label: "Volume Down" },
  Screenshot: { icon: Camera, color: "text-purple-600", label: "Screenshot" },
  ToggleCamera: { icon: Camera, color: "text-blue-600", label: "Toggle Camera" },
  ToggleMicrophone: { icon: Mic, color: "text-orange-600", label: "Toggle Microphone" },
  Settings: { icon: SettingsIcon, color: "text-gray-600", label: "Open Settings" },
  Quit: { icon: Power, color: "text-red-600", label: "Quit App" },
  Custom: { icon: Edit, color: "text-gray-600", label: "Custom Action" },
} as const;

const CATEGORY_COLORS = {
  Media: "bg-blue-100 text-blue-800",
  Recording: "bg-red-100 text-red-800",
  System: "bg-gray-100 text-gray-800",
  Custom: "bg-purple-100 text-purple-800",
} as const;

export default function HotkeyManager() {
  const [hotkeys, setHotkeys] = useState<Hotkey[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  
  useEffect(() => {
    loadDefaultHotkeys();
  }, []);

  const loadDefaultHotkeys = async () => {
    try {
      setLoading(true);
      const response = await invoke<HotkeyListResponse>("get_default_hotkeys");
      if (response.success) {
        setHotkeys(response.hotkeys);
      } else {
        setError(response.message);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load hotkeys");
    } finally {
      setLoading(false);
    }
  };

  
  const toggleHotkey = async (hotkeyId: string, enabled: boolean) => {
    try {
      const response = await invoke<HotkeyRegistrationResponse>("set_hotkey_enabled", {
        hotkeyId,
        enabled
      });

      if (response.success) {
        setHotkeys(prev =>
          prev.map(h =>
            h.id === hotkeyId
              ? { ...h, enabled }
              : h
          )
        );
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to update hotkey");
    }
  };

  const getActionIcon = (action: HotkeyAction) => {
    const actionConfig = ACTION_ICONS[action as keyof typeof ACTION_ICONS];
    const Icon = actionConfig?.icon || Edit;
    return <Icon className={`h-4 w-4 ${actionConfig?.color || "text-gray-600"}`} />;
  };

  const getCategoryBadge = (category: HotkeyCategory) => {
    const colorClass = CATEGORY_COLORS[category as keyof typeof CATEGORY_COLORS];
    return (
      <Badge variant="outline" className={colorClass}>
        {category}
      </Badge>
    );
  };

  if (loading) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>Loading hotkeys...</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="text-center py-8 text-muted-foreground">
            Loading global hotkey configuration...
          </div>
        </CardContent>
      </Card>
    );
  }

  if (error) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>Error</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="text-center py-8 text-red-600">
            Error loading hotkeys: {error}
          </div>
          <Button onClick={loadDefaultHotkeys} className="w-full mt-4">
            Retry
          </Button>
        </CardContent>
      </Card>
    );
  }

  return (
    <div className="space-y-4">
      {/* F1-F12 Quick Reference */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <span>🎮 F1-F12 Global Hotkeys</span>
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="text-sm text-muted-foreground mb-4">
            These hotkeys work even when the application is not in focus:
          </div>
          <div className="grid gap-2 md:grid-cols-2 lg:grid-cols-3">
            {[
              { key: "Ctrl+F1", action: "Toggle Mute", icon: MicOff },
              { key: "Ctrl+F2", action: "Start Video", icon: Play },
              { key: "Ctrl+F3", action: "Stop Video", icon: Pause },
              { key: "Ctrl+F4", action: "Take Screenshot", icon: Camera },
              { key: "Ctrl+F5", action: "Start Recording", icon: Camera },
              { key: "Ctrl+F6", action: "Stop Recording", icon: Camera },
              { key: "Ctrl+F7", action: "Toggle Camera", icon: Camera },
              { key: "Ctrl+F8", action: "Start Audio", icon: Play },
              { key: "Ctrl+F9", action: "Stop Audio", icon: Pause },
              { key: "Ctrl+F10", action: "Toggle Microphone", icon: Mic },
              { key: "Ctrl+F11", action: "Open Settings", icon: SettingsIcon },
              { key: "Ctrl+F12", action: "Quit Application", icon: Power },
              { key: "Shift+F11", action: "Volume Up", icon: Volume2 },
              { key: "Shift+F12", action: "Volume Down", icon: Volume2 },
            ].map((item) => (
              <div key={item.key} className="flex items-center gap-3 p-2 rounded border">
                <item.icon className="h-4 w-4 text-muted-foreground" />
                <Badge variant="outline">{item.key}</Badge>
                <span className="text-sm">{item.action}</span>
              </div>
            ))}
          </div>
        </CardContent>
      </Card>

      {/* All Hotkeys */}
      <Card>
        <CardHeader>
          <CardTitle>All Global Hotkeys</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          {hotkeys.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              No hotkeys configured
            </div>
          ) : (
            <div className="space-y-3">
              {hotkeys.map((hotkey) => (
                <div
                  key={hotkey.id}
                  className={`flex items-center justify-between p-4 border rounded-lg ${
                    hotkey.enabled ? "bg-card" : "bg-muted/50"
                  }`}
                >
                  <div className="flex items-center gap-4">
                    {/* Action Icon */}
                    <div className="flex-shrink-0">
                      {getActionIcon(hotkey.action)}
                    </div>

                    {/* Hotkey Info */}
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2 mb-1">
                        <span className="font-medium">{hotkey.name}</span>
                        {getCategoryBadge(hotkey.category)}
                        {hotkey.global && (
                          <Badge variant="outline" className="bg-green-100 text-green-800">
                            Global
                          </Badge>
                        )}
                      </div>
                      <div className="flex items-center gap-2 text-sm text-muted-foreground">
                        <Badge variant="secondary">{hotkey.key_combination}</Badge>
                        <span>•</span>
                        <span>{hotkey.description}</span>
                      </div>
                      {hotkey.is_registered && (
                        <div className="text-xs text-green-600 mt-1">
                          ✓ Registered with system
                        </div>
                      )}
                    </div>
                  </div>

                  {/* Controls */}
                  <div className="flex items-center gap-3">
                    <Switch
                      checked={hotkey.enabled}
                      onCheckedChange={(enabled) => toggleHotkey(hotkey.id, enabled)}
                      aria-label={`Toggle ${hotkey.name}`}
                    />
                    <Button
                      variant="outline"
                      size="sm"
                                          >
                      Edit
                    </Button>
                    <Button variant="outline" size="sm">
                      <Trash2 className="h-4 w-4" />
                    </Button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Add Hotkey Button */}
      <Button
        className="w-full"
              >
        <Plus className="h-4 w-4 mr-2" />
        Add Custom Hotkey
      </Button>
    </div>
  );
}