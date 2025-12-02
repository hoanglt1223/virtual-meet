import { useState, useEffect, useCallback } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Progress } from "@/components/ui/progress";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Square, Circle, Trash2, Folder, Settings, Clock } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-shell";
import type { Recording, RecordingState, AppSettings } from "@/types";

interface RecordingPreset {
  id: string;
  name: string;
  description: string;
}

interface RecordingConfig {
  video_resolution: string;
  video_quality: string;
  video_codec: string;
  frame_rate: number;
  video_bitrate: number;
  audio_quality: string;
  audio_codec: string;
  audio_bitrate: number;
  output_format: string;
}

export default function Recording() {
  // State management
  const [recordingState, setRecordingState] = useState<RecordingState>({
    isRecording: false,
    duration: 0,
    resolution: "1080p",
    quality: "medium",
  });
  const [recordings, setRecordings] = useState<Recording[]>([]);
  const [presets, setPresets] = useState<RecordingPreset[]>([]);
  const [selectedPreset, setSelectedPreset] = useState<string>("balanced_1080p");
  const [outputFolder, setOutputFolder] = useState<string>("");
  const [recordingConfig, setRecordingConfig] = useState<RecordingConfig>({
    video_resolution: "1080p",
    video_quality: "high",
    video_codec: "h264",
    frame_rate: 30,
    video_bitrate: 5000000,
    audio_quality: "standard",
    audio_codec: "aac",
    audio_bitrate: 128000,
    output_format: "mp4",
  });
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Utility functions
  const formatTime = (seconds: number) => {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    const secs = seconds % 60;
    return `${hours.toString().padStart(2, '0')}:${minutes.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
  };

  const formatFileSize = (bytes: number) => {
    if (bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + " " + sizes[i];
  };

  const generateFileName = () => {
    const now = new Date();
    const timestamp = now.toISOString().replace(/[:.]/g, '-').slice(0, -5);
    return `recording_${timestamp}.${recordingConfig.output_format}`;
  };

  // Load initial data
  const loadPresets = useCallback(async () => {
    try {
      const result = await invoke<RecordingPreset[]>("get_recording_presets");
      setPresets(result);
    } catch (err) {
      console.error("Failed to load presets:", err);
      setError("Failed to load recording presets");
    }
  }, []);

  const loadSettings = useCallback(async () => {
    try {
      const response = await invoke<any>("get_settings");
      const settings = response.settings as AppSettings;
      if (settings?.recording?.default_output_path) {
        setOutputFolder(settings.recording.default_output_path);
      }
      if (settings?.video?.default_resolution) {
        // Convert HD720p/HD1080p to 720p/1080p for UI
        const resolutionMap: Record<string, string> = {
          'HD720p': '720p',
          'HD1080p': '1080p',
          'HD1440p': '1440p',
          'UHD4K': '4k'
        };
        setRecordingConfig(prev => ({
          ...prev,
          video_resolution: resolutionMap[settings.video.default_resolution] || '1080p'
        }));
      }
      if (settings?.video?.default_quality) {
        // convert Low/Medium/High/Ultra to fast/high/ultra for UI
        const qualityMap: Record<string, string> = {
          'Low': 'fast',
          'Medium': 'balanced',
          'High': 'high',
          'Ultra': 'ultra'
        };
        setRecordingConfig(prev => ({
          ...prev,
          video_quality: qualityMap[settings.video.default_quality] || 'high'
        }));
      }
    } catch (err) {
      console.error("Failed to load settings:", err);
    }
  }, []);

  const updateRecordingStatus = useCallback(async () => {
    try {
      const status = await invoke<any>("get_recording_status");
      setRecordingState({
        isRecording: status.state.is_recording,
        startTime: status.state.start_time ? new Date(status.state.start_time) : undefined,
        duration: status.stats.duration_seconds,
        outputPath: status.state.output_path,
        resolution: recordingConfig.video_resolution,
        quality: recordingConfig.video_quality,
      });
    } catch (err) {
      console.error("Failed to get recording status:", err);
    }
  }, [recordingConfig.video_resolution, recordingConfig.video_quality]);

  // Load data on component mount
  useEffect(() => {
    loadPresets();
    loadSettings();
    updateRecordingStatus();
  }, [loadPresets, loadSettings, updateRecordingStatus]);

  // Update recording status periodically when recording
  useEffect(() => {
    if (!recordingState.isRecording) return;

    const interval = setInterval(() => {
      updateRecordingStatus();
    }, 1000);

    return () => clearInterval(interval);
  }, [recordingState.isRecording, updateRecordingStatus]);

  // Recording control functions
  const startRecording = async () => {
    if (!outputFolder) {
      setError("Please select an output folder");
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      const fileName = generateFileName();
      const outputPath = `${outputFolder}\\${fileName}`;

      // Apply preset if selected
      let config = recordingConfig;
      if (selectedPreset) {
        const preset = presets.find(p => p.id === selectedPreset);
        if (preset) {
          // Config would be updated based on preset
          // For now, we'll use the current config
        }
      }

      await invoke<string>("start_recording", {
        outputPath,
        config: config,
      });

      setRecordingState(prev => ({
        ...prev,
        isRecording: true,
        startTime: new Date(),
        duration: 0,
        outputPath,
      }));

      // Refresh recording status after starting
      setTimeout(updateRecordingStatus, 500);
    } catch (err) {
      console.error("Failed to start recording:", err);
      setError(err instanceof Error ? err.message : "Failed to start recording");
    } finally {
      setIsLoading(false);
    }
  };

  const stopRecording = async () => {
    setIsLoading(true);
    setError(null);

    try {
      await invoke("stop_recording");

      setRecordingState(prev => ({
        ...prev,
        isRecording: false,
      }));

      // Refresh recordings list after stopping
      setTimeout(() => {
        // Load recent recordings would go here
        updateRecordingStatus();
      }, 1000);
    } catch (err) {
      console.error("Failed to stop recording:", err);
      setError(err instanceof Error ? err.message : "Failed to stop recording");
    } finally {
      setIsLoading(false);
    }
  };

  const selectOutputFolder = async () => {
    try {
      const result = await invoke<string>("select_output_folder");
      if (result) {
        setOutputFolder(result);
      }
    } catch (err) {
      console.error("Failed to select folder:", err);
      setError("Failed to select output folder");
    }
  };

  const openRecordingLocation = async (recording: Recording) => {
    try {
      const path = recording.path.substring(0, recording.path.lastIndexOf('\\'));
      await open(path);
    } catch (err) {
      console.error("Failed to open location:", err);
    }
  };

  const deleteRecording = async (recordingId: string) => {
    try {
      await invoke("delete_recording", { recordingId });
      setRecordings(prev => prev.filter(r => r.id !== recordingId));
    } catch (err) {
      console.error("Failed to delete recording:", err);
      setError("Failed to delete recording");
    }
  };

  return (
    <div className="space-y-6">
      {/* Error Display */}
      {error && (
        <div className="bg-destructive/10 border border-destructive/20 rounded-lg p-3">
          <p className="text-sm text-destructive">{error}</p>
        </div>
      )}

      {/* Recording Controls */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Square className="h-5 w-5" />
            Recording Controls
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-6">
          {/* Start/Stop Controls */}
          <div className="flex items-center gap-4">
            <Button
              onClick={recordingState.isRecording ? stopRecording : startRecording}
              className={recordingState.isRecording ? "bg-destructive hover:bg-destructive/90" : "bg-primary hover:bg-primary/90"}
              disabled={isLoading}
              size="lg"
            >
              {isLoading ? (
                <>
                  <div className="animate-spin h-4 w-4 mr-2 border-2 border-current border-t-transparent rounded-full" />
                  Processing...
                </>
              ) : recordingState.isRecording ? (
                <>
                  <Square className="h-4 w-4 mr-2" />
                  Stop Recording
                </>
              ) : (
                <>
                  <Circle className="h-4 w-4 mr-2" />
                  Start Recording
                </>
              )}
            </Button>

            {recordingState.isRecording && (
              <Badge variant="destructive" className="animate-pulse flex items-center gap-1">
                <Circle className="h-2 w-2 fill-current" />
                Recording
              </Badge>
            )}

            {recordingState.outputPath && (
              <div className="text-sm text-muted-foreground">
                Saving to: <span className="font-mono">{recordingState.outputPath}</span>
              </div>
            )}
          </div>

          {/* Duration Display */}
          <div className="space-y-3 bg-muted/30 rounded-lg p-4">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <Clock className="h-4 w-4 text-muted-foreground" />
                <span className="font-medium">Recording Duration</span>
              </div>
              <span className="font-mono text-lg">{formatTime(recordingState.duration)}</span>
            </div>
            <Progress value={(recordingState.duration % 60) * 1.67} className="h-3" />
            {recordingState.startTime && (
              <div className="text-xs text-muted-foreground">
                Started: {recordingState.startTime.toLocaleString()}
              </div>
            )}
          </div>

          {/* Recording Configuration */}
          <div className="space-y-4">
            <div className="flex items-center gap-2 mb-3">
              <Settings className="h-4 w-4" />
              <span className="font-medium">Recording Configuration</span>
            </div>

            <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
              {/* Preset Selection */}
              <div className="space-y-2">
                <Label htmlFor="preset">Quality Preset</Label>
                <Select value={selectedPreset} onValueChange={setSelectedPreset}>
                  <SelectTrigger id="preset">
                    <SelectValue placeholder="Select preset" />
                  </SelectTrigger>
                  <SelectContent>
                    {presets.map((preset) => (
                      <SelectItem key={preset.id} value={preset.id}>
                        <div>
                          <div className="font-medium">{preset.name}</div>
                          <div className="text-xs text-muted-foreground">{preset.description}</div>
                        </div>
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>

              {/* Resolution Selection */}
              <div className="space-y-2">
                <Label htmlFor="resolution">Resolution</Label>
                <Select
                  value={recordingConfig.video_resolution}
                  onValueChange={(value) => setRecordingConfig(prev => ({ ...prev, video_resolution: value }))}
                >
                  <SelectTrigger id="resolution">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="480p">480p (854x480)</SelectItem>
                    <SelectItem value="720p">720p (1280x720)</SelectItem>
                    <SelectItem value="1080p">1080p (1920x1080)</SelectItem>
                  </SelectContent>
                </Select>
              </div>

              {/* Quality Selection */}
              <div className="space-y-2">
                <Label htmlFor="quality">Video Quality</Label>
                <Select
                  value={recordingConfig.video_quality}
                  onValueChange={(value) => setRecordingConfig(prev => ({ ...prev, video_quality: value }))}
                >
                  <SelectTrigger id="quality">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="fast">Fast (Low Quality)</SelectItem>
                    <SelectItem value="high">High Quality</SelectItem>
                    <SelectItem value="ultra">Ultra Quality</SelectItem>
                  </SelectContent>
                </Select>
              </div>

              {/* Frame Rate */}
              <div className="space-y-2">
                <Label htmlFor="framerate">Frame Rate</Label>
                <Select
                  value={recordingConfig.frame_rate.toString()}
                  onValueChange={(value) => setRecordingConfig(prev => ({ ...prev, frame_rate: parseInt(value) }))}
                >
                  <SelectTrigger id="framerate">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="24">24 FPS</SelectItem>
                    <SelectItem value="30">30 FPS</SelectItem>
                    <SelectItem value="60">60 FPS</SelectItem>
                  </SelectContent>
                </Select>
              </div>

              {/* Audio Quality */}
              <div className="space-y-2">
                <Label htmlFor="audio-quality">Audio Quality</Label>
                <Select
                  value={recordingConfig.audio_quality}
                  onValueChange={(value) => setRecordingConfig(prev => ({ ...prev, audio_quality: value }))}
                >
                  <SelectTrigger id="audio-quality">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="low">Low (64 kbps)</SelectItem>
                    <SelectItem value="standard">Standard (128 kbps)</SelectItem>
                    <SelectItem value="high">High (192 kbps)</SelectItem>
                  </SelectContent>
                </Select>
              </div>

              {/* Video Codec */}
              <div className="space-y-2">
                <Label htmlFor="video-codec">Video Codec</Label>
                <Select
                  value={recordingConfig.video_codec}
                  onValueChange={(value) => setRecordingConfig(prev => ({ ...prev, video_codec: value }))}
                >
                  <SelectTrigger id="video-codec">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="h264">H.264</SelectItem>
                    <SelectItem value="h265">H.265</SelectItem>
                    <SelectItem value="vp9">VP9</SelectItem>
                  </SelectContent>
                </Select>
              </div>

              {/* Audio Codec */}
              <div className="space-y-2">
                <Label htmlFor="audio-codec">Audio Codec</Label>
                <Select
                  value={recordingConfig.audio_codec}
                  onValueChange={(value) => setRecordingConfig(prev => ({ ...prev, audio_codec: value }))}
                >
                  <SelectTrigger id="audio-codec">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="aac">AAC</SelectItem>
                    <SelectItem value="mp3">MP3</SelectItem>
                    <SelectItem value="opus">Opus</SelectItem>
                  </SelectContent>
                </Select>
              </div>

              {/* Output Format */}
              <div className="space-y-2">
                <Label htmlFor="output-format">Output Format</Label>
                <Select
                  value={recordingConfig.output_format}
                  onValueChange={(value) => setRecordingConfig(prev => ({ ...prev, output_format: value }))}
                >
                  <SelectTrigger id="output-format">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="mp4">MP4</SelectItem>
                    <SelectItem value="mkv">MKV</SelectItem>
                    <SelectItem value="webm">WebM</SelectItem>
                  </SelectContent>
                </Select>
              </div>
            </div>
          </div>

          {/* Output Folder Configuration */}
          <div className="space-y-3 border rounded-lg p-4">
            <Label className="text-base font-medium">Output Folder Configuration</Label>
            <div className="flex gap-2">
              <Input
                value={outputFolder}
                onChange={(e) => setOutputFolder(e.target.value)}
                placeholder="Select or enter output folder path..."
                className="flex-1 font-mono text-sm"
              />
              <Button variant="outline" onClick={selectOutputFolder} className="flex items-center gap-2">
                <Folder className="h-4 w-4" />
                Browse
              </Button>
            </div>
            {outputFolder && (
              <div className="text-xs text-muted-foreground">
                Recordings will be saved to: <span className="font-mono">{outputFolder}</span>
              </div>
            )}
          </div>
        </CardContent>
      </Card>

      {/* Recent Recordings */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center justify-between">
            <span className="flex items-center gap-2">
              <Clock className="h-5 w-5" />
              Recent Recordings ({recordings.length})
            </span>
            <Button variant="outline" size="sm" onClick={() => window.location.reload()}>
              Refresh
            </Button>
          </CardTitle>
        </CardHeader>
        <CardContent>
          {recordings.length === 0 ? (
            <div className="text-center py-12">
              <Clock className="h-12 w-12 mx-auto text-muted-foreground mb-4" />
              <div className="text-muted-foreground mb-4">
                No recordings yet. Start recording to see them here.
              </div>
              <div className="text-sm text-muted-foreground">
                Your recordings will appear here with details about duration, file size, and quality.
              </div>
            </div>
          ) : (
            <div className="space-y-4">
              {recordings.map((recording) => (
                <div
                  key={recording.id}
                  className="flex items-center justify-between p-4 border rounded-lg hover:bg-muted/50 transition-colors"
                >
                  <div className="flex-1">
                    <div className="font-medium text-base mb-1">{recording.filename}</div>
                    <div className="flex flex-wrap items-center gap-3 text-sm text-muted-foreground mb-2">
                      <span className="flex items-center gap-1">
                        <Clock className="h-3 w-3" />
                        {formatTime(recording.duration)}
                      </span>
                      <span className="flex items-center gap-1">
                        <Settings className="h-3 w-3" />
                        {recording.resolution}
                      </span>
                      <span className="flex items-center gap-1">
                        <Badge variant="secondary" className="text-xs">
                          {recording.quality}
                        </Badge>
                      </span>
                      <span>{formatFileSize(recording.fileSize)}</span>
                    </div>
                    <div className="text-xs text-muted-foreground">
                      Created: {new Date(recording.createdAt).toLocaleString()}
                    </div>
                  </div>
                  <div className="flex gap-2 ml-4">
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => openRecordingLocation(recording)}
                      className="flex items-center gap-2"
                    >
                      <Folder className="h-4 w-4" />
                      Open
                    </Button>
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => deleteRecording(recording.id)}
                      className="flex items-center gap-2 text-destructive hover:text-destructive"
                    >
                      <Trash2 className="h-4 w-4" />
                      Delete
                    </Button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}