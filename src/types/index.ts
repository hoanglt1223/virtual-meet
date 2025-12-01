export interface MediaDevice {
  id: string;
  name: string;
  type: 'webcam' | 'microphone' | 'speaker';
  isVirtual: boolean;
  isAvailable: boolean;
}

export interface MediaFile {
  id: string;
  name: string;
  path: string;
  type: 'video' | 'audio';
  duration?: number;
  thumbnailPath?: string;
  metadata?: {
    width?: number;
    height?: number;
    fps?: number;
    audioChannels?: number;
    sampleRate?: number;
    bitrate?: number;
  };
  createdAt: Date;
}

export interface Recording {
  id: string;
  filename: string;
  path: string;
  duration: number;
  fileSize: number;
  resolution: string;
  quality: string;
  createdAt: Date;
}

export interface Hotkey {
  id: string;
  keys: string;
  action: 'playVideo' | 'playAudio' | 'stopVideo' | 'stopAudio' | 'startRecording' | 'stopRecording' | 'executeScript';
  targetId?: string; // media file or script ID
  isActive: boolean;
}

export interface Script {
  id: string;
  name: string;
  description: string;
  content: string;
  isActive: boolean;
  lastRun?: Date;
}

export interface AppSettings {
  mediaFolders: string[];
  videoDeviceId?: string;
  audioDeviceId?: string;
  recording: {
    defaultResolution: '720p' | '1080p';
    defaultQuality: 'low' | 'medium' | 'high';
    outputPath: string;
  };
  ui: {
    theme: 'light' | 'dark' | 'system';
    autoStartRecording: boolean;
    showPreview: boolean;
  };
}

export interface PlaybackState {
  isPlaying: boolean;
  currentVideo?: MediaFile;
  currentAudio?: MediaFile;
  position: number;
  volume: number;
  isLooping: boolean;
}

export interface RecordingState {
  isRecording: boolean;
  startTime?: Date;
  duration: number;
  outputPath?: string;
  resolution: string;
  quality: string;
}