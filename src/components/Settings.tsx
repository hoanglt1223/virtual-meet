import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import {
  FolderOpen,
  Monitor,
  Settings as SettingsIcon,
  Download,
  Upload,
  RotateCcw,
  Check,
  AlertTriangle,
  Info,
  Volume2,
  Video,
  HardDrive,
  Zap,
  Shield,
  Palette
} from "lucide-react";
import HotkeyManager from "@/components/HotkeyManager";
import type { AppSettings } from "@/types";

// Device info interfaces matching Rust structs
interface VideoDeviceInfo {
  id: string;
  name: string;
  resolution: [number, number];
  fps: number;
  is_virtual: boolean;
  capabilities: string[];
}

interface AudioDeviceInfo {
  id: string;
  name: string;
  sample_rate: number;
  channels: number;
  bit_depth: number;
  is_virtual: boolean;
}

export default function Settings() {
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [videoDevices, setVideoDevices] = useState<VideoDeviceInfo[]>([]);
  const [audioDevices, setAudioDevices] = useState<AudioDeviceInfo[]>([]);
  const [loading, _setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<{ type: 'success' | 'error' | 'info'; text: string } | null>(null);

  // Load settings and devices on component mount
  useEffect(() => {
    loadSettings();
    loadDevices();
  }, []);

  const loadSettings = async () => {
    try {
      const response = await invoke('get_settings');
      if (response && typeof response === 'object' && 'success' in response) {
        const settingsResponse = response as { success: boolean; settings?: AppSettings };
        if (settingsResponse.success && settingsResponse.settings) {
          setSettings(settingsResponse.settings);
        }
      }
    } catch (error) {
      console.error('Failed to load settings:', error);
      showMessage('Failed to load settings', 'error');
    }
  };

  const loadDevices = async () => {
    try {
      const [videoResponse, audioResponse] = await Promise.all([
        invoke('get_available_video_devices'),
        invoke('get_available_audio_devices')
      ]);

      if (Array.isArray(videoResponse)) {
        setVideoDevices(videoResponse as VideoDeviceInfo[]);
      }
      if (Array.isArray(audioResponse)) {
        setAudioDevices(audioResponse as AudioDeviceInfo[]);
      }
    } catch (error) {
      console.error('Failed to load devices:', error);
    }
  };

  const updateSettings = async (category: string, updatedSettings: any) => {
    if (!settings) return;

    setSaving(true);
    try {
      const response = await invoke('update_settings', {
        request: {
          category,
          settings: updatedSettings
        }
      });

      if (response && typeof response === 'object' && 'is_valid' in response) {
        const validationResponse = response as { is_valid: boolean; errors?: string[]; warnings?: string[] };

        if (validationResponse.is_valid) {
          // Update local state
          setSettings(prev => prev ? { ...prev, [category.toLowerCase()]: updatedSettings } : null);
          showMessage('Settings updated successfully', 'success');
        } else {
          showMessage(`Validation failed: ${validationResponse.errors?.join(', ')}`, 'error');
        }
      }
    } catch (error) {
      console.error('Failed to update settings:', error);
      showMessage('Failed to update settings', 'error');
    } finally {
      setSaving(false);
    }
  };

  const selectOutputFolder = async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false
      });

      if (selected && typeof selected === 'string' && settings) {
        const updatedRecording = {
          ...settings.recording,
          default_output_path: selected
        };
        await updateSettings('Recording', updatedRecording);
      }
    } catch (error) {
      console.error('Failed to select folder:', error);
      showMessage('Failed to select folder', 'error');
    }
  };

  const resetSettings = async (categories: string[]) => {
    try {
      const response = await invoke('reset_settings', {
        request: {
          categories,
          confirm: true
        }
      });

      if (response && typeof response === 'object' && 'success' in response) {
        const resetResponse = response as { success: boolean; message: string };
        if (resetResponse.success) {
          await loadSettings();
          showMessage('Settings reset successfully', 'success');
        } else {
          showMessage(resetResponse.message, 'error');
        }
      }
    } catch (error) {
      console.error('Failed to reset settings:', error);
      showMessage('Failed to reset settings', 'error');
    }
  };

  const exportSettings = async () => {
    try {
      const selected = await open({
        directory: false,
        filters: [{
          name: 'JSON',
          extensions: ['json']
        }]
      });

      if (selected && typeof selected === 'string') {
        const response = await invoke('export_settings', {
          request: {
            file_path: selected,
            include_sensitive: false
          }
        });

        if (response && typeof response === 'object' && 'success' in response) {
          const exportResponse = response as { success: boolean; message: string };
          if (exportResponse.success) {
            showMessage('Settings exported successfully', 'success');
          } else {
            showMessage(exportResponse.message, 'error');
          }
        }
      }
    } catch (error) {
      console.error('Failed to export settings:', error);
      showMessage('Failed to export settings', 'error');
    }
  };

  const importSettings = async () => {
    try {
      const selected = await open({
        directory: false,
        filters: [{
          name: 'JSON',
          extensions: ['json']
        }]
      });

      if (selected && typeof selected === 'string') {
        const response = await invoke('import_settings', { file_path: selected });

        if (response && typeof response === 'object' && 'is_valid' in response) {
          const importResponse = response as {
            is_valid: boolean;
            errors?: string[];
            warnings?: string[]
          };

          if (importResponse.is_valid) {
            await loadSettings();
            showMessage('Settings imported successfully', 'success');
          } else {
            showMessage(`Import failed: ${importResponse.errors?.join(', ')}`, 'error');
          }
        }
      }
    } catch (error) {
      console.error('Failed to import settings:', error);
      showMessage('Failed to import settings', 'error');
    }
  };

  const showMessage = (text: string, type: 'success' | 'error' | 'info') => {
    setMessage({ text, type });
    setTimeout(() => setMessage(null), 5000);
  };

  if (loading || !settings) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-lg text-muted-foreground">Loading settings...</div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Status Messages */}
      {message && (
        <Alert className={`${
          message.type === 'success' ? 'border-green-200 bg-green-50' :
          message.type === 'error' ? 'border-red-200 bg-red-50' :
          'border-blue-200 bg-blue-50'
        }`}>
          {message.type === 'success' && <Check className="h-4 w-4" />}
          {message.type === 'error' && <AlertTriangle className="h-4 w-4" />}
          {message.type === 'info' && <Info className="h-4 w-4" />}
          <AlertDescription>{message.text}</AlertDescription>
        </Alert>
      )}

      <Tabs defaultValue="devices" className="space-y-4">
        <div className="flex items-center justify-between">
          <TabsList className="grid w-full grid-cols-8">
            <TabsTrigger value="devices" className="flex items-center gap-2">
              <Monitor className="h-4 w-4" />
              Devices
            </TabsTrigger>
            <TabsTrigger value="video" className="flex items-center gap-2">
              <Video className="h-4 w-4" />
              Video
            </TabsTrigger>
            <TabsTrigger value="audio" className="flex items-center gap-2">
              <Volume2 className="h-4 w-4" />
              Audio
            </TabsTrigger>
            <TabsTrigger value="recording" className="flex items-center gap-2">
              <HardDrive className="h-4 w-4" />
              Recording
            </TabsTrigger>
            <TabsTrigger value="hotkeys" className="flex items-center gap-2">
              <Zap className="h-4 w-4" />
              Hotkeys
            </TabsTrigger>
            <TabsTrigger value="general" className="flex items-center gap-2">
              <SettingsIcon className="h-4 w-4" />
              General
            </TabsTrigger>
            <TabsTrigger value="ui" className="flex items-center gap-2">
              <Palette className="h-4 w-4" />
              UI
            </TabsTrigger>
            <TabsTrigger value="advanced" className="flex items-center gap-2">
              <Shield className="h-4 w-4" />
              Advanced
            </TabsTrigger>
          </TabsList>

          <div className="flex items-center gap-2">
            <Button variant="outline" size="sm" onClick={exportSettings}>
              <Download className="h-4 w-4 mr-2" />
              Export
            </Button>
            <Button variant="outline" size="sm" onClick={importSettings}>
              <Upload className="h-4 w-4 mr-2" />
              Import
            </Button>
          </div>
        </div>

        {/* Device Settings */}
        <TabsContent value="devices" className="space-y-4">
          <div className="grid gap-6 md:grid-cols-2">
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <Monitor className="h-5 w-5" />
                  Webcam Selection
                </CardTitle>
              </CardHeader>
              <CardContent className="space-y-4">
                <div>
                  <Label htmlFor="webcam-select">Preferred Webcam</Label>
                  <Select
                    value={settings.devices.preferred_webcam || ""}
                    onValueChange={async (value) => {
                      const updatedDevices = { ...settings.devices, preferred_webcam: value || undefined };
                      await updateSettings('Devices', updatedDevices);
                    }}
                  >
                    <SelectTrigger>
                      <SelectValue placeholder="Select webcam device" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="">None</SelectItem>
                      {videoDevices.map((device) => (
                        <SelectItem key={device.id} value={device.id}>
                          <div className="flex items-center justify-between w-full">
                            <span>{device.name}</span>
                            {device.is_virtual && (
                              <Badge variant="secondary" className="ml-2">Virtual</Badge>
                            )}
                          </div>
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>

                <div className="space-y-2">
                  <Label htmlFor="mic-select">Preferred Microphone</Label>
                  <Select
                    value={settings.devices.preferred_microphone || ""}
                    onValueChange={async (value) => {
                      const updatedDevices = { ...settings.devices, preferred_microphone: value || undefined };
                      await updateSettings('Devices', updatedDevices);
                    }}
                  >
                    <SelectTrigger>
                      <SelectValue placeholder="Select microphone device" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="">None</SelectItem>
                      {audioDevices.map((device) => (
                        <SelectItem key={device.id} value={device.id}>
                          <div className="flex items-center justify-between w-full">
                            <span>{device.name}</span>
                            {device.is_virtual && (
                              <Badge variant="secondary" className="ml-2">Virtual</Badge>
                            )}
                          </div>
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>

                <div className="flex items-center justify-between">
                  <Label htmlFor="auto-detect">Auto-detect Devices</Label>
                  <Switch
                    id="auto-detect"
                    checked={settings.devices.auto_detect_devices}
                    onCheckedChange={async (checked) => {
                      const updatedDevices = { ...settings.devices, auto_detect_devices: checked };
                      await updateSettings('Devices', updatedDevices);
                    }}
                  />
                </div>

                <div>
                  <Label htmlFor="refresh-interval">Device Refresh Interval (seconds)</Label>
                  <Input
                    id="refresh-interval"
                    type="number"
                    value={settings.devices.device_refresh_interval}
                    onChange={async (e) => {
                      const updatedDevices = {
                        ...settings.devices,
                        device_refresh_interval: parseInt(e.target.value) || 30
                      };
                      await updateSettings('Devices', updatedDevices);
                    }}
                    min="5"
                    max="300"
                  />
                </div>
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <SettingsIcon className="h-5 w-5" />
                  Virtual Device Settings
                </CardTitle>
              </CardHeader>
              <CardContent className="space-y-4">
                <div>
                  <Label htmlFor="webcam-backend">Webcam Backend</Label>
                  <Select
                    value={settings.devices.virtual_device_settings.webcam_backend}
                    onValueChange={async (value) => {
                      const updatedVirtual = {
                        ...settings.devices.virtual_device_settings,
                        webcam_backend: value
                      };
                      const updatedDevices = { ...settings.devices, virtual_device_settings: updatedVirtual };
                      await updateSettings('Devices', updatedDevices);
                    }}
                  >
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="DirectShow">DirectShow</SelectItem>
                      <SelectItem value="MediaFoundation">Media Foundation</SelectItem>
                    </SelectContent>
                  </Select>
                </div>

                <div>
                  <Label htmlFor="mic-backend">Microphone Backend</Label>
                  <Select
                    value={settings.devices.virtual_device_settings.microphone_backend}
                    onValueChange={async (value) => {
                      const updatedVirtual = {
                        ...settings.devices.virtual_device_settings,
                        microphone_backend: value
                      };
                      const updatedDevices = { ...settings.devices, virtual_device_settings: updatedVirtual };
                      await updateSettings('Devices', updatedDevices);
                    }}
                  >
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="WASAPI">WASAPI</SelectItem>
                      <SelectItem value="DirectSound">DirectSound</SelectItem>
                      <SelectItem value="ASIO">ASIO</SelectItem>
                    </SelectContent>
                  </Select>
                </div>

                <div>
                  <Label htmlFor="buffer-size">Buffer Size (MB)</Label>
                  <Input
                    id="buffer-size"
                    type="number"
                    value={settings.devices.virtual_device_settings.buffer_size_mb}
                    onChange={async (e) => {
                      const updatedVirtual = {
                        ...settings.devices.virtual_device_settings,
                        buffer_size_mb: parseInt(e.target.value) || 256
                      };
                      const updatedDevices = { ...settings.devices, virtual_device_settings: updatedVirtual };
                      await updateSettings('Devices', updatedDevices);
                    }}
                    min="64"
                    max="1024"
                    step="64"
                  />
                </div>

                <div className="flex items-center justify-between">
                  <Label htmlFor="low-latency">Low Latency Mode</Label>
                  <Switch
                    id="low-latency"
                    checked={settings.devices.virtual_device_settings.low_latency_mode}
                    onCheckedChange={async (checked) => {
                      const updatedVirtual = {
                        ...settings.devices.virtual_device_settings,
                        low_latency_mode: checked
                      };
                      const updatedDevices = { ...settings.devices, virtual_device_settings: updatedVirtual };
                      await updateSettings('Devices', updatedDevices);
                    }}
                  />
                </div>
              </CardContent>
            </Card>
          </div>
        </TabsContent>

        {/* Video Settings */}
        <TabsContent value="video" className="space-y-4">
          <div className="grid gap-6 md:grid-cols-2">
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <Video className="h-5 w-5" />
                  Video Configuration
                </CardTitle>
              </CardHeader>
              <CardContent className="space-y-4">
                <div>
                  <Label htmlFor="resolution">Default Resolution</Label>
                  <Select
                    value={settings.video.default_resolution}
                    onValueChange={async (value) => {
                      const updatedVideo = { ...settings.video, default_resolution: value as any };
                      await updateSettings('Video', updatedVideo);
                    }}
                  >
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="HD720p">720p (1280x720)</SelectItem>
                      <SelectItem value="HD1080p">1080p (1920x1080)</SelectItem>
                      <SelectItem value="HD1440p">1440p (2560x1440)</SelectItem>
                      <SelectItem value="UHD4K">4K (3840x2160)</SelectItem>
                    </SelectContent>
                  </Select>
                </div>

                <div>
                  <Label htmlFor="fps">Default Frame Rate (FPS)</Label>
                  <Input
                    id="fps"
                    type="number"
                    value={settings.video.default_fps}
                    onChange={async (e) => {
                      const updatedVideo = {
                        ...settings.video,
                        default_fps: parseFloat(e.target.value) || 30.0
                      };
                      await updateSettings('Video', updatedVideo);
                    }}
                    min="15"
                    max="120"
                    step="0.1"
                  />
                </div>

                <div>
                  <Label htmlFor="quality">Default Quality</Label>
                  <Select
                    value={settings.video.default_quality}
                    onValueChange={async (value) => {
                      const updatedVideo = { ...settings.video, default_quality: value as any };
                      await updateSettings('Video', updatedVideo);
                    }}
                  >
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="Low">Low</SelectItem>
                      <SelectItem value="Medium">Medium</SelectItem>
                      <SelectItem value="High">High</SelectItem>
                      <SelectItem value="Ultra">Ultra</SelectItem>
                    </SelectContent>
                  </Select>
                </div>

                <div className="flex items-center justify-between">
                  <Label htmlFor="hardware-accel">Hardware Acceleration</Label>
                  <Switch
                    id="hardware-accel"
                    checked={settings.video.hardware_acceleration}
                    onCheckedChange={async (checked) => {
                      const updatedVideo = { ...settings.video, hardware_acceleration: checked };
                      await updateSettings('Video', updatedVideo);
                    }}
                  />
                </div>
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <SettingsIcon className="h-5 w-5" />
                  Advanced Video Settings
                </CardTitle>
              </CardHeader>
              <CardContent className="space-y-4">
                <div>
                  <Label htmlFor="video-backend">Video Backend</Label>
                  <Select
                    value={settings.video.video_backend}
                    onValueChange={async (value) => {
                      const updatedVideo = { ...settings.video, video_backend: value };
                      await updateSettings('Video', updatedVideo);
                    }}
                  >
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="DirectShow">DirectShow</SelectItem>
                      <SelectItem value="MediaFoundation">Media Foundation</SelectItem>
                    </SelectContent>
                  </Select>
                </div>

                <div>
                  <Label htmlFor="color-space">Color Space</Label>
                  <Select
                    value={settings.video.color_space}
                    onValueChange={async (value) => {
                      const updatedVideo = { ...settings.video, color_space: value };
                      await updateSettings('Video', updatedVideo);
                    }}
                  >
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="RGB24">RGB24</SelectItem>
                      <SelectItem value="YUV420P">YUV420P</SelectItem>
                      <SelectItem value="YUV444P">YUV444P</SelectItem>
                      <SelectItem value="Auto">Auto</SelectItem>
                    </SelectContent>
                  </Select>
                </div>

                <div className="flex items-center justify-between">
                  <Label htmlFor="deinterlacing">Deinterlacing</Label>
                  <Switch
                    id="deinterlacing"
                    checked={settings.video.deinterlacing}
                    onCheckedChange={async (checked) => {
                      const updatedVideo = { ...settings.video, deinterlacing: checked };
                      await updateSettings('Video', updatedVideo);
                    }}
                  />
                </div>
              </CardContent>
            </Card>
          </div>
        </TabsContent>

        {/* Audio Settings */}
        <TabsContent value="audio" className="space-y-4">
          <div className="grid gap-6 md:grid-cols-2">
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <Volume2 className="h-5 w-5" />
                  Audio Configuration
                </CardTitle>
              </CardHeader>
              <CardContent className="space-y-4">
                <div>
                  <Label htmlFor="sample-rate">Default Sample Rate (Hz)</Label>
                  <Select
                    value={settings.audio.default_sample_rate.toString()}
                    onValueChange={async (value) => {
                      const updatedAudio = {
                        ...settings.audio,
                        default_sample_rate: parseInt(value) || 44100
                      };
                      await updateSettings('Audio', updatedAudio);
                    }}
                  >
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="44100">44.1 kHz</SelectItem>
                      <SelectItem value="48000">48 kHz</SelectItem>
                      <SelectItem value="96000">96 kHz</SelectItem>
                      <SelectItem value="192000">192 kHz</SelectItem>
                    </SelectContent>
                  </Select>
                </div>

                <div>
                  <Label htmlFor="bit-depth">Default Bit Depth (bits)</Label>
                  <Select
                    value={settings.audio.default_bit_depth.toString()}
                    onValueChange={async (value) => {
                      const updatedAudio = {
                        ...settings.audio,
                        default_bit_depth: parseInt(value) || 16
                      };
                      await updateSettings('Audio', updatedAudio);
                    }}
                  >
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="16">16 bits</SelectItem>
                      <SelectItem value="24">24 bits</SelectItem>
                      <SelectItem value="32">32 bits</SelectItem>
                    </SelectContent>
                  </Select>
                </div>

                <div>
                  <Label htmlFor="channels">Default Channels</Label>
                  <Select
                    value={settings.audio.default_channels.toString()}
                    onValueChange={async (value) => {
                      const updatedAudio = {
                        ...settings.audio,
                        default_channels: parseInt(value) || 2
                      };
                      await updateSettings('Audio', updatedAudio);
                    }}
                  >
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="1">Mono (1)</SelectItem>
                      <SelectItem value="2">Stereo (2)</SelectItem>
                    </SelectContent>
                  </Select>
                </div>

                <div>
                  <Label htmlFor="audio-backend">Audio Backend</Label>
                  <Select
                    value={settings.audio.default_audio_backend}
                    onValueChange={async (value) => {
                      const updatedAudio = { ...settings.audio, default_audio_backend: value };
                      await updateSettings('Audio', updatedAudio);
                    }}
                  >
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="WASAPI">WASAPI</SelectItem>
                      <SelectItem value="DirectSound">DirectSound</SelectItem>
                      <SelectItem value="ASIO">ASIO</SelectItem>
                    </SelectContent>
                  </Select>
                </div>

                <div>
                  <Label htmlFor="buffer-size-audio">Audio Buffer Size</Label>
                  <Input
                    id="buffer-size-audio"
                    type="number"
                    value={settings.audio.audio_buffer_size}
                    onChange={async (e) => {
                      const updatedAudio = {
                        ...settings.audio,
                        audio_buffer_size: parseInt(e.target.value) || 1024
                      };
                      await updateSettings('Audio', updatedAudio);
                    }}
                    min="128"
                    max="8192"
                    step="128"
                  />
                </div>
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <Zap className="h-5 w-5" />
                  Audio Enhancement
                </CardTitle>
              </CardHeader>
              <CardContent className="space-y-4">
                <div className="flex items-center justify-between">
                  <Label htmlFor="noise-reduction">Noise Reduction</Label>
                  <Switch
                    id="noise-reduction"
                    checked={settings.audio.noise_reduction}
                    onCheckedChange={async (checked) => {
                      const updatedAudio = { ...settings.audio, noise_reduction: checked };
                      await updateSettings('Audio', updatedAudio);
                    }}
                  />
                </div>

                <div className="flex items-center justify-between">
                  <Label htmlFor="echo-cancellation">Echo Cancellation</Label>
                  <Switch
                    id="echo-cancellation"
                    checked={settings.audio.echo_cancellation}
                    onCheckedChange={async (checked) => {
                      const updatedAudio = { ...settings.audio, echo_cancellation: checked };
                      await updateSettings('Audio', updatedAudio);
                    }}
                  />
                </div>

                <div className="flex items-center justify-between">
                  <Label htmlFor="auto-gain">Auto Gain Control</Label>
                  <Switch
                    id="auto-gain"
                    checked={settings.audio.auto_gain_control}
                    onCheckedChange={async (checked) => {
                      const updatedAudio = { ...settings.audio, auto_gain_control: checked };
                      await updateSettings('Audio', updatedAudio);
                    }}
                  />
                </div>
              </CardContent>
            </Card>
          </div>
        </TabsContent>

        {/* Recording Settings */}
        <TabsContent value="recording" className="space-y-4">
          <div className="grid gap-6 md:grid-cols-2">
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <HardDrive className="h-5 w-5" />
                  Recording Configuration
                </CardTitle>
              </CardHeader>
              <CardContent className="space-y-4">
                <div>
                  <Label htmlFor="output-path">Output Path</Label>
                  <div className="flex gap-2">
                    <Input
                      id="output-path"
                      value={settings.recording.default_output_path}
                      readOnly
                      className="flex-1"
                    />
                    <Button variant="outline" onClick={selectOutputFolder}>
                      <FolderOpen className="h-4 w-4" />
                    </Button>
                  </div>
                </div>

                <div>
                  <Label htmlFor="format">Default Format</Label>
                  <Select
                    value={settings.recording.default_format}
                    onValueChange={async (value) => {
                      const updatedRecording = { ...settings.recording, default_format: value as any };
                      await updateSettings('Recording', updatedRecording);
                    }}
                  >
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="MP4">MP4</SelectItem>
                      <SelectItem value="MKV">MKV</SelectItem>
                      <SelectItem value="WebM">WebM</SelectItem>
                      <SelectItem value="AVI">AVI</SelectItem>
                    </SelectContent>
                  </Select>
                </div>

                <div>
                  <Label htmlFor="compression">Compression Level ({settings.recording.compression_level})</Label>
                  <Input
                    id="compression"
                    type="range"
                    min="0"
                    max="9"
                    value={settings.recording.compression_level}
                    onChange={async (e) => {
                      const updatedRecording = {
                        ...settings.recording,
                        compression_level: parseInt(e.target.value)
                      };
                      await updateSettings('Recording', updatedRecording);
                    }}
                    className="w-full"
                  />
                  <div className="flex justify-between text-xs text-muted-foreground">
                    <span>Fast (Large)</span>
                    <span>Slow (Small)</span>
                  </div>
                </div>
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <SettingsIcon className="h-5 w-5" />
                  Recording Options
                </CardTitle>
              </CardHeader>
              <CardContent className="space-y-4">
                <div className="flex items-center justify-between">
                  <Label htmlFor="auto-segment">Auto Segment Files</Label>
                  <Switch
                    id="auto-segment"
                    checked={settings.recording.auto_segment_files}
                    onCheckedChange={async (checked) => {
                      const updatedRecording = { ...settings.recording, auto_segment_files: checked };
                      await updateSettings('Recording', updatedRecording);
                    }}
                  />
                </div>

                {settings.recording.auto_segment_files && (
                  <div>
                    <Label htmlFor="segment-duration">Segment Duration (minutes)</Label>
                    <Input
                      id="segment-duration"
                      type="number"
                      value={settings.recording.segment_duration_minutes}
                      onChange={async (e) => {
                        const updatedRecording = {
                          ...settings.recording,
                          segment_duration_minutes: parseInt(e.target.value) || 60
                        };
                        await updateSettings('Recording', updatedRecording);
                      }}
                      min="1"
                      max="1440"
                    />
                  </div>
                )}

                <div className="flex items-center justify-between">
                  <Label htmlFor="include-timestamp">Include Timestamp</Label>
                  <Switch
                    id="include-timestamp"
                    checked={settings.recording.include_timestamp}
                    onCheckedChange={async (checked) => {
                      const updatedRecording = { ...settings.recording, include_timestamp: checked };
                      await updateSettings('Recording', updatedRecording);
                    }}
                  />
                </div>

                <div className="flex items-center justify-between">
                  <Label htmlFor="simultaneous">Simultaneous Recording</Label>
                  <Switch
                    id="simultaneous"
                    checked={settings.recording.simultaneous_recording}
                    onCheckedChange={async (checked) => {
                      const updatedRecording = { ...settings.recording, simultaneous_recording: checked };
                      await updateSettings('Recording', updatedRecording);
                    }}
                  />
                </div>
              </CardContent>
            </Card>
          </div>
        </TabsContent>

        {/* Hotkeys */}
        <TabsContent value="hotkeys" className="space-y-4">
          <HotkeyManager />
        </TabsContent>

        {/* General Settings */}
        <TabsContent value="general" className="space-y-4">
          <div className="grid gap-6 md:grid-cols-2">
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <SettingsIcon className="h-5 w-5" />
                  Application Behavior
                </CardTitle>
              </CardHeader>
              <CardContent className="space-y-4">
                <div className="flex items-center justify-between">
                  <Label htmlFor="auto-start">Start with Windows</Label>
                  <Switch
                    id="auto-start"
                    checked={settings.general.auto_start}
                    onCheckedChange={async (checked) => {
                      const updatedGeneral = { ...settings.general, auto_start: checked };
                      await updateSettings('General', updatedGeneral);
                    }}
                  />
                </div>

                <div className="flex items-center justify-between">
                  <Label htmlFor="start-minimized">Start Minimized</Label>
                  <Switch
                    id="start-minimized"
                    checked={settings.general.start_minimized}
                    onCheckedChange={async (checked) => {
                      const updatedGeneral = { ...settings.general, start_minimized: checked };
                      await updateSettings('General', updatedGeneral);
                    }}
                  />
                </div>

                <div className="flex items-center justify-between">
                  <Label htmlFor="minimize-tray">Minimize to Tray</Label>
                  <Switch
                    id="minimize-tray"
                    checked={settings.general.minimize_to_tray}
                    onCheckedChange={async (checked) => {
                      const updatedGeneral = { ...settings.general, minimize_to_tray: checked };
                      await updateSettings('General', updatedGeneral);
                    }}
                  />
                </div>

                <div className="flex items-center justify-between">
                  <Label htmlFor="check-updates">Check for Updates</Label>
                  <Switch
                    id="check-updates"
                    checked={settings.general.check_updates}
                    onCheckedChange={async (checked) => {
                      const updatedGeneral = { ...settings.general, check_updates: checked };
                      await updateSettings('General', updatedGeneral);
                    }}
                  />
                </div>

                <div>
                  <Label htmlFor="auto-save">Auto-save Interval (minutes)</Label>
                  <Input
                    id="auto-save"
                    type="number"
                    value={settings.general.auto_save_interval}
                    onChange={async (e) => {
                      const updatedGeneral = {
                        ...settings.general,
                        auto_save_interval: parseInt(e.target.value) || 5
                      };
                      await updateSettings('General', updatedGeneral);
                    }}
                    min="1"
                    max="60"
                  />
                </div>
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <Palette className="h-5 w-5" />
                  Appearance
                </CardTitle>
              </CardHeader>
              <CardContent className="space-y-4">
                <div>
                  <Label htmlFor="theme">Theme</Label>
                  <Select
                    value={settings.general.theme}
                    onValueChange={async (value) => {
                      const updatedGeneral = { ...settings.general, theme: value as any };
                      await updateSettings('General', updatedGeneral);
                    }}
                  >
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="Light">Light</SelectItem>
                      <SelectItem value="Dark">Dark</SelectItem>
                      <SelectItem value="System">System</SelectItem>
                    </SelectContent>
                  </Select>
                </div>

                <div>
                  <Label htmlFor="language">Language</Label>
                  <Select
                    value={settings.general.language}
                    onValueChange={async (value) => {
                      const updatedGeneral = { ...settings.general, language: value };
                      await updateSettings('General', updatedGeneral);
                    }}
                  >
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="en">English</SelectItem>
                      <SelectItem value="zh">中文</SelectItem>
                      <SelectItem value="ja">日本語</SelectItem>
                      <SelectItem value="es">Español</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              </CardContent>
            </Card>
          </div>
        </TabsContent>

        {/* UI Settings */}
        <TabsContent value="ui" className="space-y-4">
          <div className="grid gap-6 md:grid-cols-2">
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <Palette className="h-5 w-5" />
                  Window Behavior
                </CardTitle>
              </CardHeader>
              <CardContent className="space-y-4">
                <div className="flex items-center justify-between">
                  <Label htmlFor="always-on-top">Always on Top</Label>
                  <Switch
                    id="always-on-top"
                    checked={settings.ui.always_on_top}
                    onCheckedChange={async (checked) => {
                      const updatedUI = { ...settings.ui, always_on_top: checked };
                      await updateSettings('UI', updatedUI);
                    }}
                  />
                </div>

                <div className="flex items-center justify-between">
                  <Label htmlFor="show-tooltips">Show Tooltips</Label>
                  <Switch
                    id="show-tooltips"
                    checked={settings.ui.show_tooltips}
                    onCheckedChange={async (checked) => {
                      const updatedUI = { ...settings.ui, show_tooltips: checked };
                      await updateSettings('UI', updatedUI);
                    }}
                  />
                </div>

                <div className="flex items-center justify-between">
                  <Label htmlFor="show-notifications">Show Notifications</Label>
                  <Switch
                    id="show-notifications"
                    checked={settings.ui.show_notifications}
                    onCheckedChange={async (checked) => {
                      const updatedUI = { ...settings.ui, show_notifications: checked };
                      await updateSettings('UI', updatedUI);
                    }}
                  />
                </div>

                <div className="flex items-center justify-between">
                  <Label htmlFor="compact-mode">Compact Mode</Label>
                  <Switch
                    id="compact-mode"
                    checked={settings.ui.compact_mode}
                    onCheckedChange={async (checked) => {
                      const updatedUI = { ...settings.ui, compact_mode: checked };
                      await updateSettings('UI', updatedUI);
                    }}
                  />
                </div>
              </CardContent>
            </Card>
          </div>
        </TabsContent>

        {/* Advanced Settings */}
        <TabsContent value="advanced" className="space-y-4">
          <div className="grid gap-6 md:grid-cols-2">
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <Shield className="h-5 w-5" />
                  Logging & Debugging
                </CardTitle>
              </CardHeader>
              <CardContent className="space-y-4">
                <div>
                  <Label htmlFor="log-level">Log Level</Label>
                  <Select
                    value={settings.advanced.log_level}
                    onValueChange={async (value) => {
                      const updatedAdvanced = { ...settings.advanced, log_level: value as any };
                      await updateSettings('Advanced', updatedAdvanced);
                    }}
                  >
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="Error">Error</SelectItem>
                      <SelectItem value="Warn">Warning</SelectItem>
                      <SelectItem value="Info">Info</SelectItem>
                      <SelectItem value="Debug">Debug</SelectItem>
                      <SelectItem value="Trace">Trace</SelectItem>
                    </SelectContent>
                  </Select>
                </div>

                <div>
                  <Label htmlFor="max-log-size">Max Log Size (MB)</Label>
                  <Input
                    id="max-log-size"
                    type="number"
                    value={settings.advanced.max_log_size_mb}
                    onChange={async (e) => {
                      const updatedAdvanced = {
                        ...settings.advanced,
                        max_log_size_mb: parseInt(e.target.value) || 100
                      };
                      await updateSettings('Advanced', updatedAdvanced);
                    }}
                    min="10"
                    max="1000"
                    step="10"
                  />
                </div>

                <div className="flex items-center justify-between">
                  <Label htmlFor="debug-mode">Debug Mode</Label>
                  <Switch
                    id="debug-mode"
                    checked={settings.advanced.debug_mode}
                    onCheckedChange={async (checked) => {
                      const updatedAdvanced = { ...settings.advanced, debug_mode: checked };
                      await updateSettings('Advanced', updatedAdvanced);
                    }}
                  />
                </div>

                <div className="flex items-center justify-between">
                  <Label htmlFor="experimental">Experimental Features</Label>
                  <Switch
                    id="experimental"
                    checked={settings.advanced.experimental_features}
                    onCheckedChange={async (checked) => {
                      const updatedAdvanced = { ...settings.advanced, experimental_features: checked };
                      await updateSettings('Advanced', updatedAdvanced);
                    }}
                  />
                </div>
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <Zap className="h-5 w-5" />
                  Performance & System
                </CardTitle>
              </CardHeader>
              <CardContent className="space-y-4">
                <div>
                  <Label htmlFor="performance">Performance Mode</Label>
                  <Select
                    value={settings.advanced.performance_mode}
                    onValueChange={async (value) => {
                      const updatedAdvanced = { ...settings.advanced, performance_mode: value };
                      await updateSettings('Advanced', updatedAdvanced);
                    }}
                  >
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="PowerSaving">Power Saving</SelectItem>
                      <SelectItem value="Balanced">Balanced</SelectItem>
                      <SelectItem value="HighPerformance">High Performance</SelectItem>
                    </SelectContent>
                  </Select>
                </div>

                <div>
                  <Label htmlFor="ffmpeg-path">Custom FFmpeg Path (Optional)</Label>
                  <Input
                    id="ffmpeg-path"
                    value={settings.advanced.custom_ffmpeg_path || ""}
                    onChange={async (e) => {
                      const updatedAdvanced = {
                        ...settings.advanced,
                        custom_ffmpeg_path: e.target.value || undefined
                      };
                      await updateSettings('Advanced', updatedAdvanced);
                    }}
                    placeholder="Leave empty for built-in FFmpeg"
                  />
                </div>

                <div className="pt-4 space-y-2">
                  <Button
                    variant="outline"
                    className="w-full"
                    onClick={() => resetSettings(['Advanced'])}
                  >
                    <RotateCcw className="h-4 w-4 mr-2" />
                    Reset Advanced Settings
                  </Button>

                  <Button
                    variant="outline"
                    className="w-full"
                    onClick={() => resetSettings(['General', 'Video', 'Audio', 'Recording', 'Devices', 'Hotkeys', 'UI', 'Advanced'])}
                  >
                    <RotateCcw className="h-4 w-4 mr-2" />
                    Reset All Settings
                  </Button>
                </div>
              </CardContent>
            </Card>
          </div>
        </TabsContent>
      </Tabs>

      {saving && (
        <div className="fixed bottom-4 right-4 bg-blue-500 text-white px-4 py-2 rounded-md shadow-lg">
          Saving settings...
        </div>
      )}
    </div>
  );
}