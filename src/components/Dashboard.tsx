import { useState, useRef, useEffect } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Progress } from "@/components/ui/progress";
import {
  Play, Pause, Square, SkipBack, SkipForward, Mic, Video, Volume2,
  VolumeX, Maximize2, Settings, Camera, MicOff, Monitor, Download,
  RefreshCw, WifiOff, CheckCircle, Circle
} from "lucide-react";
import type { MediaFile, PlaybackState, RecordingState } from "@/types";

export default function Dashboard() {
  const [playbackState, setPlaybackState] = useState<PlaybackState>({
    isPlaying: false,
    position: 0,
    volume: 0.75,
    isLooping: false,
  });

  const [recordingState, setRecordingState] = useState<RecordingState>({
    isRecording: false,
    duration: 0,
    resolution: "1080p",
    quality: "high",
  });

  const [selectedVideo, _setSelectedVideo] = useState<MediaFile | null>(null);
  const [selectedAudio, _setSelectedAudio] = useState<MediaFile | null>(null);
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [isMuted, setIsMuted] = useState(false);
  const [connectionStatus, _setConnectionStatus] = useState<'connected' | 'disconnected' | 'connecting'>('connected');

  const videoRef = useRef<HTMLVideoElement>(null);
  const audioRef = useRef<HTMLAudioElement>(null);

  // Mock duration updates for demo
  useEffect(() => {
    const interval = setInterval(() => {
      setRecordingState((prev: RecordingState) => ({
        ...prev,
        duration: prev.isRecording ? prev.duration + 1 : prev.duration,
      }));

      if (playbackState.isPlaying && selectedVideo) {
        setPlaybackState(prev => ({
          ...prev,
          position: Math.min((prev.position + 1) % (selectedVideo.duration || 100), selectedVideo.duration || 100),
        }));
      }
    }, 1000);

    return () => clearInterval(interval);
  }, [recordingState.isRecording, playbackState.isPlaying, selectedVideo]);

  const formatTime = (seconds: number): string => {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    const secs = seconds % 60;
    return `${hours.toString().padStart(2, '0')}:${minutes.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
  };

  const handlePlayPause = () => {
    setPlaybackState(prev => ({
      ...prev,
      isPlaying: !prev.isPlaying,
    }));

    if (videoRef.current) {
      if (playbackState.isPlaying) {
        videoRef.current.pause();
      } else {
        videoRef.current.play();
      }
    }
  };

  const handleStop = () => {
    setPlaybackState(prev => ({
      ...prev,
      isPlaying: false,
      position: 0,
    }));

    if (videoRef.current) {
      videoRef.current.pause();
      videoRef.current.currentTime = 0;
    }
  };

  const handleSeek = (direction: 'forward' | 'backward') => {
    const increment = direction === 'forward' ? 10 : -10;
    setPlaybackState(prev => ({
      ...prev,
      position: Math.max(0, Math.min(prev.position + increment, selectedVideo?.duration || 100)),
    }));

    if (videoRef.current) {
      videoRef.current.currentTime += increment;
    }
  };

  const handleVolumeChange = (volume: number) => {
    setPlaybackState(prev => ({
      ...prev,
      volume,
      isPlaying: volume === 0 ? false : prev.isPlaying,
    }));

    if (videoRef.current && audioRef.current) {
      videoRef.current.volume = volume;
      audioRef.current.volume = volume;
    }
  };

  const handleMuteToggle = () => {
    setIsMuted(prev => !prev);
    handleVolumeChange(isMuted ? playbackState.volume : 0);
  };

  const handleRecordingToggle = () => {
    setRecordingState((prev: RecordingState) => ({
      ...prev,
      isRecording: !prev.isRecording,
      startTime: !prev.isRecording ? new Date() : prev.startTime,
    }));
  };

  const handleFullscreen = () => {
    if (videoRef.current) {
      if (!isFullscreen) {
        videoRef.current.requestFullscreen();
      } else {
        document.exitFullscreen();
      }
    }
    setIsFullscreen(prev => !prev);
  };

  const getStatusIcon = () => {
    switch (connectionStatus) {
      case 'connected':
        return <CheckCircle className="h-4 w-4 text-green-500" />;
      case 'disconnected':
        return <WifiOff className="h-4 w-4 text-red-500" />;
      case 'connecting':
        return <RefreshCw className="h-4 w-4 text-yellow-500 animate-spin" />;
    }
  };

  return (
    <div className="space-y-6">
      {/* Status Bar */}
      <Card>
        <CardContent className="py-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-6">
              <div className="flex items-center gap-2">
                {getStatusIcon()}
                <span className="text-sm font-medium capitalize">{connectionStatus}</span>
              </div>
              <Badge variant={recordingState.isRecording ? "destructive" : "secondary"}>
                {recordingState.isRecording ? (
                  <><Circle className="h-3 w-3 mr-1 animate-pulse" /> Recording</>
                ) : (
                  "Ready"
                )}
              </Badge>
              <Badge variant={playbackState.isPlaying ? "default" : "secondary"}>
                {playbackState.isPlaying ? (
                  <><Play className="h-3 w-3 mr-1" /> Playing</>
                ) : (
                  "Paused"
                )}
              </Badge>
            </div>
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              {selectedVideo && `${selectedVideo.name} • ${selectedVideo.metadata?.width || 'N/A'}x${selectedVideo.metadata?.height || 'N/A'}`}
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Main Video Preview Area */}
      <Card className="md:col-span-2">
        <CardHeader>
          <CardTitle className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <Video className="h-5 w-5" />
              Real-time Video Preview
            </div>
            <div className="flex items-center gap-2">
              <Button size="sm" variant="outline" onClick={handleFullscreen}>
                <Maximize2 className="h-4 w-4 mr-2" />
                Fullscreen
              </Button>
            </div>
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="relative aspect-video bg-black rounded-lg overflow-hidden group">
            {selectedVideo ? (
              <video
                ref={videoRef}
                className="w-full h-full object-contain"
                src={selectedVideo.path}
                onTimeUpdate={(e) => setPlaybackState(prev => ({ ...prev, position: Math.floor(e.currentTarget.currentTime) }))}
              />
            ) : (
              <div className="flex items-center justify-center h-full bg-gradient-to-br from-gray-900 to-gray-800">
                <div className="text-center space-y-4">
                  <Monitor className="h-16 w-16 mx-auto text-gray-600" />
                  <div>
                    <p className="text-gray-400 font-medium">No video source selected</p>
                    <p className="text-gray-500 text-sm">Select a video from the Media Library to begin</p>
                  </div>
                  <Button variant="outline" size="sm">
                    <Camera className="h-4 w-4 mr-2" />
                    Select Video Source
                  </Button>
                </div>
              </div>
            )}

            {/* Overlay Controls */}
            <div className="absolute bottom-0 left-0 right-0 bg-gradient-to-t from-black/80 to-transparent p-4 opacity-0 group-hover:opacity-100 transition-opacity">
              <div className="flex items-center gap-4 text-white">
                <Button size="sm" variant="secondary" onClick={handlePlayPause}>
                  {playbackState.isPlaying ? <Pause className="h-4 w-4" /> : <Play className="h-4 w-4" />}
                </Button>
                <Button size="sm" variant="secondary" onClick={() => handleSeek('backward')}>
                  <SkipBack className="h-4 w-4" />
                </Button>
                <Button size="sm" variant="secondary" onClick={() => handleSeek('forward')}>
                  <SkipForward className="h-4 w-4" />
                </Button>
                <Button size="sm" variant="secondary" onClick={handleStop}>
                  <Square className="h-4 w-4" />
                </Button>

                <div className="flex-1">
                  <Progress
                    value={(playbackState.position / (selectedVideo?.duration || 100)) * 100}
                    className="h-2"
                  />
                  <div className="flex justify-between text-xs mt-1 text-white/70">
                    <span>{formatTime(playbackState.position)}</span>
                    <span>{formatTime(selectedVideo?.duration || 0)}</span>
                  </div>
                </div>
              </div>
            </div>
          </div>

          {/* Video Status Info */}
          <div className="grid grid-cols-4 gap-4 text-sm">
            <div className="bg-muted rounded-lg p-3">
              <div className="flex items-center gap-2 text-muted-foreground">
                <Camera className="h-4 w-4" />
                Resolution
              </div>
              <div className="font-medium mt-1">
                {selectedVideo?.metadata?.width && selectedVideo?.metadata?.height
                  ? `${selectedVideo.metadata.width}x${selectedVideo.metadata.height}`
                  : "N/A"
                }
              </div>
            </div>
            <div className="bg-muted rounded-lg p-3">
              <div className="flex items-center gap-2 text-muted-foreground">
                <Video className="h-4 w-4" />
                Frame Rate
              </div>
              <div className="font-medium mt-1">
                {selectedVideo?.metadata?.fps ? `${selectedVideo.metadata.fps} fps` : "N/A"}
              </div>
            </div>
            <div className="bg-muted rounded-lg p-3">
              <div className="flex items-center gap-2 text-muted-foreground">
                <Settings className="h-4 w-4" />
                Quality
              </div>
              <div className="font-medium mt-1">{recordingState.quality}</div>
            </div>
            <div className="bg-muted rounded-lg p-3">
              <div className="flex items-center gap-2 text-muted-foreground">
                <Monitor className="h-4 w-4" />
                Output
              </div>
              <div className="font-medium mt-1">{recordingState.resolution}</div>
            </div>
          </div>
        </CardContent>
      </Card>

      <div className="grid gap-6 md:grid-cols-2">
        {/* Playback Controls */}
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Play className="h-5 w-5" />
              Playback Controls
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="flex items-center gap-2">
              <Button onClick={handlePlayPause} className="flex-1">
                {playbackState.isPlaying ? (
                  <><Pause className="h-4 w-4 mr-2" /> Pause</>
                ) : (
                  <><Play className="h-4 w-4 mr-2" /> Play</>
                )}
              </Button>
              <Button variant="outline" onClick={handleStop}>
                <Square className="h-4 w-4 mr-2" />
                Stop
              </Button>
            </div>

            <div className="flex items-center gap-2">
              <Button variant="outline" size="sm" onClick={() => handleSeek('backward')}>
                <SkipBack className="h-4 w-4 mr-2" />
                -10s
              </Button>
              <div className="flex-1">
                <Progress
                  value={(playbackState.position / (selectedVideo?.duration || 100)) * 100}
                  className="h-2"
                />
                <div className="flex justify-between text-xs mt-1 text-muted-foreground">
                  <span>{formatTime(playbackState.position)}</span>
                  <span>{formatTime(selectedVideo?.duration || 0)}</span>
                </div>
              </div>
              <Button variant="outline" size="sm" onClick={() => handleSeek('forward')}>
                +10s
                <SkipForward className="h-4 w-4 ml-2" />
              </Button>
            </div>

            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <span className="text-sm font-medium">Volume</span>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={handleMuteToggle}
                  className="h-6 w-6 p-0"
                >
                  {isMuted || playbackState.volume === 0 ? (
                    <VolumeX className="h-4 w-4" />
                  ) : (
                    <Volume2 className="h-4 w-4" />
                  )}
                </Button>
              </div>
              <div className="flex items-center gap-2">
                <Progress
                  value={playbackState.volume * 100}
                  className="flex-1 h-2 cursor-pointer"
                  onClick={(e) => {
                    const rect = e.currentTarget.getBoundingClientRect();
                    const percent = (e.clientX - rect.left) / rect.width;
                    handleVolumeChange(Math.max(0, Math.min(1, percent)));
                  }}
                />
                <span className="text-sm text-muted-foreground w-10 text-right">
                  {Math.round(playbackState.volume * 100)}%
                </span>
              </div>
            </div>
          </CardContent>
        </Card>

        {/* Audio Status Card */}
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Mic className="h-5 w-5" />
              Audio Status
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="bg-muted rounded-lg p-8 text-center">
              {selectedAudio ? (
                <div className="space-y-2">
                  <Volume2 className="h-12 w-12 mx-auto text-primary" />
                  <p className="font-medium">{selectedAudio.name}</p>
                  <p className="text-sm text-muted-foreground">
                    {selectedAudio.metadata?.sampleRate ? `${selectedAudio.metadata.sampleRate} Hz` : "N/A"} •
                    {selectedAudio.metadata?.audioChannels ? ` ${selectedAudio.metadata.audioChannels} channels` : ""}
                  </p>
                </div>
              ) : (
                <div className="space-y-2">
                  <MicOff className="h-12 w-12 mx-auto text-muted-foreground" />
                  <p className="text-muted-foreground">No audio selected</p>
                </div>
              )}
            </div>

            <audio ref={audioRef} src={selectedAudio?.path} />

            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <span className="text-sm font-medium">Audio Level</span>
                {isMuted || playbackState.volume === 0 ? (
                  <Badge variant="destructive">
                    <VolumeX className="h-3 w-3 mr-1" />
                    Muted
                  </Badge>
                ) : (
                  <span className="text-sm text-muted-foreground">
                    {Math.round(playbackState.volume * 100)}%
                  </span>
                )}
              </div>
              <Progress value={isMuted ? 0 : playbackState.volume * 100} className="h-2" />
            </div>
          </CardContent>
        </Card>
      </div>

      <div className="grid gap-6 md:grid-cols-2">
        {/* Recording Status */}
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Circle className="h-5 w-5" />
              Recording Control
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="flex items-center justify-between">
              <div className="space-y-1">
                <Badge variant={recordingState.isRecording ? "destructive" : "secondary"}>
                  {recordingState.isRecording ? (
                    <><Circle className="h-3 w-3 mr-1 animate-pulse" /> Recording</>
                  ) : (
                    "Ready to Recording"
                  )}
                </Badge>
                {recordingState.isRecording && (
                  <p className="text-sm text-muted-foreground">
                    Duration: {formatTime(recordingState.duration)}
                  </p>
                )}
              </div>
              <Button onClick={handleRecordingToggle} variant={recordingState.isRecording ? "destructive" : "default"}>
                {recordingState.isRecording ? (
                  <><Square className="h-4 w-4 mr-2" /> Stop Recording</>
                ) : (
                  <><Circle className="h-4 w-4 mr-2" /> Start Recording</>
                )}
              </Button>
            </div>

            <div className="grid grid-cols-2 gap-4 text-sm">
              <div>
                <span className="text-muted-foreground">Resolution:</span>
                <div className="font-medium">{recordingState.resolution}</div>
              </div>
              <div>
                <span className="text-muted-foreground">Quality:</span>
                <div className="font-medium capitalize">{recordingState.quality}</div>
              </div>
            </div>
          </CardContent>
        </Card>

        {/* Quick Actions */}
        <Card>
          <CardHeader>
            <CardTitle>Quick Actions</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="grid grid-cols-2 gap-3">
              <Button variant="outline" className="h-16 flex-col">
                <Camera className="h-5 w-5 mb-1" />
                Select Video
              </Button>
              <Button variant="outline" className="h-16 flex-col">
                <Mic className="h-5 w-5 mb-1" />
                Select Audio
              </Button>
              <Button variant="outline" className="h-16 flex-col">
                <Download className="h-5 w-5 mb-1" />
                Export
              </Button>
              <Button variant="outline" className="h-16 flex-col">
                <Settings className="h-5 w-5 mb-1" />
                Settings
              </Button>
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Hidden audio element for playback */}
      {selectedAudio && (
        <audio
          ref={audioRef}
          src={selectedAudio.path}
          onTimeUpdate={(e) => setPlaybackState(prev => ({ ...prev, position: Math.floor(e.currentTarget.currentTime) }))}
        />
      )}
    </div>
  );
}