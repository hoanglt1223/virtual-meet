import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Progress } from "@/components/ui/progress";
import {
  Play, Square, Mic, Video, Volume2,
  VolumeX, Camera, MicOff, Monitor,
  RefreshCw, CheckCircle, AlertCircle, Loader2
} from "lucide-react";

// Response types matching Rust structs
interface VideoResponse {
  success: boolean;
  message: string;
  video_info?: {
    width: number;
    height: number;
    frame_rate: number;
    duration?: { secs: number; nanos: number };
  };
  buffer_status?: {
    current_frames: number;
    capacity: number;
    total_processed: number;
  };
}

interface DevicesResponse {
  success: boolean;
  devices: string[];
}

interface StatusResponse {
  is_active: boolean;
  current_source?: string;
  video_info?: {
    width: number;
    height: number;
    frame_rate: number;
    duration?: { secs: number; nanos: number };
  };
}

interface AudioStatusResponse {
  is_active: boolean;
  current_source?: string;
  volume: number;
  is_muted: boolean;
}

interface WebcamModeInfo {
  id: string;
  name: string;
  description: string;
  available: boolean;
  requires: string[];
}

export default function Dashboard() {
  // Video state
  const [videoPath, setVideoPath] = useState<string | null>(null);
  const [videoStreaming, setVideoStreaming] = useState(false);
  const [videoInfo, setVideoInfo] = useState<VideoResponse["video_info"]>(undefined);
  const [videoDevices, setVideoDevices] = useState<string[]>([]);
  const [selectedVideoDevice, setSelectedVideoDevice] = useState<string>("");
  const [webcamModes, setWebcamModes] = useState<WebcamModeInfo[]>([]);
  const [selectedMode, setSelectedMode] = useState<string>("obs");

  // Audio state
  const [audioPath, setAudioPath] = useState<string | null>(null);
  const [audioStreaming, setAudioStreaming] = useState(false);
  const [audioDevices, setAudioDevices] = useState<string[]>([]);
  const [selectedAudioDevice, setSelectedAudioDevice] = useState<string>("");
  const [volume, setVolume] = useState(0.75);
  const [isMuted, setIsMuted] = useState(false);

  // UI state
  const [loading, setLoading] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [statusMessage, setStatusMessage] = useState<string>("");

  // Load devices on mount
  useEffect(() => {
    loadDevices();
    pollStatus();
    const interval = setInterval(pollStatus, 3000);
    return () => clearInterval(interval);
  }, []);

  const loadDevices = async () => {
    try {
      const videoResp = await invoke<DevicesResponse>("list_video_devices");
      if (videoResp.success) setVideoDevices(videoResp.devices);

      const audioResp = await invoke<DevicesResponse>("list_audio_devices");
      if (audioResp.success) setAudioDevices(audioResp.devices);

      const modes = await invoke<WebcamModeInfo[]>("get_webcam_modes");
      setWebcamModes(modes);
    } catch (e) {
      console.error("Failed to load devices:", e);
    }
  };

  const pollStatus = async () => {
    try {
      const webcamStatus = await invoke<StatusResponse>("get_webcam_status");
      setVideoStreaming(webcamStatus.is_active);
      if (webcamStatus.video_info) setVideoInfo(webcamStatus.video_info);
      if (webcamStatus.current_source && !videoPath) {
        setVideoPath(webcamStatus.current_source);
      }

      const micStatus = await invoke<AudioStatusResponse>("get_microphone_status");
      setAudioStreaming(micStatus.is_active);
      setVolume(micStatus.volume);
      setIsMuted(micStatus.is_muted);
      if (micStatus.current_source && !audioPath) {
        setAudioPath(micStatus.current_source);
      }
    } catch {
      // Status polling failures are non-critical
    }
  };

  // Select video file
  const handleSelectVideo = async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: "Video", extensions: ["mp4", "mkv", "avi", "webm", "mov"] }],
      });
      if (selected) {
        setVideoPath(selected as string);
        setError(null);
        setStatusMessage(`Video selected: ${(selected as string).split(/[\\/]/).pop()}`);
      }
    } catch (e) {
      setError(`Failed to select video: ${e}`);
    }
  };

  // Select audio file
  const handleSelectAudio = async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: "Audio", extensions: ["mp3", "wav", "m4a", "aac", "ogg", "flac"] }],
      });
      if (selected) {
        setAudioPath(selected as string);
        setError(null);
        setStatusMessage(`Audio selected: ${(selected as string).split(/[\\/]/).pop()}`);
      }
    } catch (e) {
      setError(`Failed to select audio: ${e}`);
    }
  };

  // Start/stop video streaming
  const handleVideoToggle = async () => {
    if (videoStreaming) {
      setLoading("video-stop");
      try {
        const resp = await invoke<VideoResponse>("stop_streaming");
        setVideoStreaming(false);
        setVideoInfo(undefined);
        setStatusMessage(resp.message);
      } catch (e) {
        setError(`Failed to stop video: ${e}`);
      }
      setLoading(null);
    } else {
      if (!videoPath) {
        setError("Select a video file first");
        return;
      }
      setLoading("video-start");
      setError(null);
      try {
        const resp = await invoke<VideoResponse>("start_streaming", { request: { path: videoPath } });
        if (resp.success) {
          setVideoStreaming(true);
          if (resp.video_info) setVideoInfo(resp.video_info);
          setStatusMessage(resp.message);
        } else {
          setError(resp.message);
        }
      } catch (e) {
        setError(`Failed to start video: ${e}`);
      }
      setLoading(null);
    }
  };

  // Start/stop audio streaming
  const handleAudioToggle = async () => {
    if (audioStreaming) {
      setLoading("audio-stop");
      try {
        const resp = await invoke<VideoResponse>("stop_audio_streaming");
        setAudioStreaming(false);
        setStatusMessage(resp.message);
      } catch (e) {
        setError(`Failed to stop audio: ${e}`);
      }
      setLoading(null);
    } else {
      if (!audioPath) {
        setError("Select an audio file first");
        return;
      }
      setLoading("audio-start");
      setError(null);
      try {
        const resp = await invoke<VideoResponse>("start_audio_streaming", { request: { path: audioPath } });
        if (resp.success) {
          setAudioStreaming(true);
          setStatusMessage(resp.message);
        } else {
          setError(resp.message);
        }
      } catch (e) {
        setError(`Failed to start audio: ${e}`);
      }
      setLoading(null);
    }
  };

  // Volume control
  const handleVolumeChange = async (newVolume: number) => {
    const clamped = Math.max(0, Math.min(1, newVolume));
    setVolume(clamped);
    try {
      await invoke("set_microphone_volume", { request: { volume: clamped } });
    } catch {
      // Non-critical
    }
  };

  const handleMuteToggle = async () => {
    const newMuted = !isMuted;
    setIsMuted(newMuted);
    try {
      await invoke("toggle_microphone_mute");
    } catch {
      // Non-critical
    }
  };

  const formatDuration = (dur?: { secs: number; nanos: number }) => {
    if (!dur) return "N/A";
    const totalSecs = dur.secs;
    const mins = Math.floor(totalSecs / 60);
    const secs = totalSecs % 60;
    return `${mins}:${secs.toString().padStart(2, "0")}`;
  };

  const fileName = (path: string | null) => path?.split(/[\\/]/).pop() || "None";

  return (
    <div className="space-y-6">
      {/* Status Bar */}
      <Card>
        <CardContent className="py-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-4">
              <Badge variant={videoStreaming ? "default" : "secondary"}>
                {videoStreaming ? (
                  <><Video className="h-3 w-3 mr-1" /> Video Active</>
                ) : (
                  <><Video className="h-3 w-3 mr-1" /> Video Off</>
                )}
              </Badge>
              <Badge variant={audioStreaming ? "default" : "secondary"}>
                {audioStreaming ? (
                  <><Mic className="h-3 w-3 mr-1" /> Audio Active</>
                ) : (
                  <><MicOff className="h-3 w-3 mr-1" /> Audio Off</>
                )}
              </Badge>
            </div>
            <div className="flex items-center gap-2">
              {statusMessage && (
                <span className="text-xs text-muted-foreground">{statusMessage}</span>
              )}
              <Button size="sm" variant="ghost" onClick={() => { loadDevices(); pollStatus(); }}>
                <RefreshCw className="h-3 w-3" />
              </Button>
            </div>
          </div>
          {error && (
            <div className="mt-2 flex items-center gap-2 text-sm text-destructive">
              <AlertCircle className="h-4 w-4" />
              {error}
              <Button size="sm" variant="ghost" className="h-5 px-1 text-xs" onClick={() => setError(null)}>
                dismiss
              </Button>
            </div>
          )}
        </CardContent>
      </Card>

      <div className="grid gap-6 md:grid-cols-2">
        {/* Video Control */}
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Camera className="h-5 w-5" />
              Virtual Webcam
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            {/* Video file selection */}
            <div className="space-y-2">
              <label className="text-sm font-medium">Video File</label>
              <div className="flex gap-2">
                <Button variant="outline" className="flex-1 justify-start truncate" onClick={handleSelectVideo}>
                  <Video className="h-4 w-4 mr-2 shrink-0" />
                  <span className="truncate">{videoPath ? fileName(videoPath) : "Select video file..."}</span>
                </Button>
              </div>
            </div>

            {/* Webcam mode selection */}
            {webcamModes.length > 0 && (
              <div className="space-y-2">
                <label className="text-sm font-medium">Mode</label>
                <select
                  className="w-full rounded-md border bg-background px-3 py-2 text-sm"
                  value={selectedMode}
                  onChange={(e) => setSelectedMode(e.target.value)}
                  disabled={videoStreaming}
                >
                  {webcamModes.map((m) => (
                    <option key={m.id} value={m.id} disabled={!m.available}>
                      {m.name}{!m.available ? " (not installed)" : ""}
                    </option>
                  ))}
                </select>
              </div>
            )}

            {/* Video device selection (OBS mode only) */}
            {selectedMode === "obs" && videoDevices.length > 0 && (
              <div className="space-y-2">
                <label className="text-sm font-medium">Target Device</label>
                <select
                  className="w-full rounded-md border bg-background px-3 py-2 text-sm"
                  value={selectedVideoDevice}
                  onChange={(e) => setSelectedVideoDevice(e.target.value)}
                >
                  <option value="">OBS Virtual Camera (default)</option>
                  {videoDevices.map((d) => (
                    <option key={d} value={d}>{d}</option>
                  ))}
                </select>
              </div>
            )}

            {/* Video info */}
            {videoInfo && (
              <div className="grid grid-cols-3 gap-2 text-xs">
                <div className="bg-muted rounded p-2">
                  <div className="text-muted-foreground">Resolution</div>
                  <div className="font-medium">{videoInfo.width}x{videoInfo.height}</div>
                </div>
                <div className="bg-muted rounded p-2">
                  <div className="text-muted-foreground">FPS</div>
                  <div className="font-medium">{videoInfo.frame_rate.toFixed(1)}</div>
                </div>
                <div className="bg-muted rounded p-2">
                  <div className="text-muted-foreground">Duration</div>
                  <div className="font-medium">{formatDuration(videoInfo.duration)}</div>
                </div>
              </div>
            )}

            {/* Start/Stop button */}
            <Button
              className="w-full"
              variant={videoStreaming ? "destructive" : "default"}
              onClick={handleVideoToggle}
              disabled={!!loading}
            >
              {loading?.startsWith("video") ? (
                <Loader2 className="h-4 w-4 mr-2 animate-spin" />
              ) : videoStreaming ? (
                <Square className="h-4 w-4 mr-2" />
              ) : (
                <Play className="h-4 w-4 mr-2" />
              )}
              {videoStreaming ? "Stop Webcam" : "Start Webcam"}
            </Button>

            {videoStreaming && (
              <div className="flex items-center gap-2 text-sm">
                <CheckCircle className="h-4 w-4 text-green-500" />
                <span className="text-green-600">Streaming to virtual camera</span>
              </div>
            )}
          </CardContent>
        </Card>

        {/* Audio Control */}
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Mic className="h-5 w-5" />
              Virtual Microphone
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            {/* Audio file selection */}
            <div className="space-y-2">
              <label className="text-sm font-medium">Audio File</label>
              <div className="flex gap-2">
                <Button variant="outline" className="flex-1 justify-start truncate" onClick={handleSelectAudio}>
                  <Mic className="h-4 w-4 mr-2 shrink-0" />
                  <span className="truncate">{audioPath ? fileName(audioPath) : "Select audio file..."}</span>
                </Button>
              </div>
            </div>

            {/* Audio device selection */}
            {audioDevices.length > 0 && (
              <div className="space-y-2">
                <label className="text-sm font-medium">Output Device</label>
                <select
                  className="w-full rounded-md border bg-background px-3 py-2 text-sm"
                  value={selectedAudioDevice}
                  onChange={(e) => setSelectedAudioDevice(e.target.value)}
                >
                  <option value="">Default output device</option>
                  {audioDevices.map((d) => (
                    <option key={d} value={d}>{d}</option>
                  ))}
                </select>
                <p className="text-xs text-muted-foreground">
                  Select "CABLE Input" (VB-Audio) to use as virtual microphone
                </p>
              </div>
            )}

            {/* Volume control */}
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <span className="text-sm font-medium">Volume</span>
                <Button variant="ghost" size="sm" onClick={handleMuteToggle} className="h-6 w-6 p-0">
                  {isMuted ? <VolumeX className="h-4 w-4" /> : <Volume2 className="h-4 w-4" />}
                </Button>
              </div>
              <div className="flex items-center gap-2">
                <Progress
                  value={isMuted ? 0 : volume * 100}
                  className="flex-1 h-2 cursor-pointer"
                  onClick={(e) => {
                    const rect = e.currentTarget.getBoundingClientRect();
                    const percent = (e.clientX - rect.left) / rect.width;
                    handleVolumeChange(percent);
                  }}
                />
                <span className="text-sm text-muted-foreground w-10 text-right">
                  {isMuted ? "Muted" : `${Math.round(volume * 100)}%`}
                </span>
              </div>
            </div>

            {/* Start/Stop button */}
            <Button
              className="w-full"
              variant={audioStreaming ? "destructive" : "default"}
              onClick={handleAudioToggle}
              disabled={!!loading}
            >
              {loading?.startsWith("audio") ? (
                <Loader2 className="h-4 w-4 mr-2 animate-spin" />
              ) : audioStreaming ? (
                <Square className="h-4 w-4 mr-2" />
              ) : (
                <Play className="h-4 w-4 mr-2" />
              )}
              {audioStreaming ? "Stop Audio" : "Start Audio"}
            </Button>

            {audioStreaming && (
              <div className="flex items-center gap-2 text-sm">
                <CheckCircle className="h-4 w-4 text-green-500" />
                <span className="text-green-600">Playing to output device</span>
              </div>
            )}
          </CardContent>
        </Card>
      </div>

      {/* Quick Info */}
      <Card>
        <CardContent className="py-4">
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
            <div className="bg-muted rounded-lg p-3">
              <div className="flex items-center gap-2 text-muted-foreground">
                <Camera className="h-4 w-4" />
                Video Source
              </div>
              <div className="font-medium mt-1 truncate">{videoPath ? fileName(videoPath) : "None"}</div>
            </div>
            <div className="bg-muted rounded-lg p-3">
              <div className="flex items-center gap-2 text-muted-foreground">
                <Mic className="h-4 w-4" />
                Audio Source
              </div>
              <div className="font-medium mt-1 truncate">{audioPath ? fileName(audioPath) : "None"}</div>
            </div>
            <div className="bg-muted rounded-lg p-3">
              <div className="flex items-center gap-2 text-muted-foreground">
                <Monitor className="h-4 w-4" />
                Video Device
              </div>
              <div className="font-medium mt-1 truncate">{selectedVideoDevice || "OBS Virtual Camera"}</div>
            </div>
            <div className="bg-muted rounded-lg p-3">
              <div className="flex items-center gap-2 text-muted-foreground">
                <Volume2 className="h-4 w-4" />
                Audio Device
              </div>
              <div className="font-medium mt-1 truncate">{selectedAudioDevice || "Default output"}</div>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
