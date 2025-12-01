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

// Hotkey action types - matches Rust HotkeyAction enum
export type HotkeyAction =
  | 'StartVideo'
  | 'StopVideo'
  | 'StartAudio'
  | 'StopAudio'
  | 'StartRecording'
  | 'StopRecording'
  | 'ToggleMute'
  | 'VolumeUp'
  | 'VolumeDown'
  | 'SwitchVideo'
  | 'SwitchAudio'
  | 'Screenshot'
  | 'ToggleCamera'
  | 'ToggleMicrophone'
  | 'Settings'
  | 'Quit'
  | 'Custom';

// Hotkey category types - matches Rust HotkeyCategory enum
export type HotkeyCategory = 'Media' | 'Recording' | 'System' | 'Custom';

export interface Hotkey {
  id: string;
  name: string;
  description: string;
  key_combination: string; // e.g., "Ctrl+F1", "Alt+F12"
  action: HotkeyAction;
  enabled: boolean;
  global: boolean; // Works when app is unfocused
  category: HotkeyCategory;
  is_registered?: boolean; // If actually registered with OS
  last_triggered?: string; // ISO timestamp
  trigger_count?: number;
}

export interface HotkeyStats {
  trigger_count: number;
  last_triggered?: string; // ISO timestamp
  average_trigger_interval_ms?: number;
}

export interface HotkeyRegistrationRequest {
  id: string;
  name: string;
  description: string;
  key_combination: string;
  action: HotkeyAction;
  global: boolean;
  enabled: boolean;
}

export interface HotkeyRegistrationResponse {
  success: boolean;
  message: string;
  hotkey_id?: string;
}

export interface HotkeyListResponse {
  success: boolean;
  message: string;
  hotkeys: Hotkey[];
}

export interface HotkeyStatusResponse {
  success: boolean;
  hotkey_id: string;
  is_registered: boolean;
  is_enabled: boolean;
  last_triggered?: string;
  trigger_count: number;
}

export interface HotkeyConflictResponse {
  success: boolean;
  has_conflict: boolean;
  conflicting_hotkeys: Hotkey[];
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