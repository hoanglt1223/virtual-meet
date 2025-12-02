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
  general: {
    auto_start: boolean;
    start_minimized: boolean;
    minimize_to_tray: boolean;
    check_updates: boolean;
    language: string;
    theme: 'Light' | 'Dark' | 'System';
    auto_save_interval: number;
  };
  video: {
    default_resolution: 'HD720p' | 'HD1080p' | 'HD1440p' | 'UHD4K';
    default_fps: number;
    default_quality: 'Low' | 'Medium' | 'High' | 'Ultra';
    hardware_acceleration: boolean;
    video_backend: string;
    deinterlacing: boolean;
    color_space: string;
  };
  audio: {
    default_sample_rate: number;
    default_bit_depth: number;
    default_channels: number;
    default_audio_backend: string;
    audio_buffer_size: number;
    noise_reduction: boolean;
    echo_cancellation: boolean;
    auto_gain_control: boolean;
  };
  recording: {
    default_output_path: string;
    default_format: 'MP4' | 'MKV' | 'WebM' | 'AVI';
    auto_segment_files: boolean;
    segment_duration_minutes: number;
    include_timestamp: boolean;
    compression_level: number;
    simultaneous_recording: boolean;
  };
  devices: {
    preferred_webcam?: string;
    preferred_microphone?: string;
    preferred_speaker?: string;
    auto_detect_devices: boolean;
    device_refresh_interval: number;
    virtual_device_settings: {
      webcam_backend: string;
      microphone_backend: string;
      buffer_size_mb: number;
      low_latency_mode: boolean;
    };
  };
  hotkeys: {
    enabled: boolean;
    global_hotkeys: Record<string, string>;
    conflict_resolution: string;
  };
  ui: {
    window_size: [number, number];
    window_position?: [number, number];
    always_on_top: boolean;
    show_tooltips: boolean;
    show_notifications: boolean;
    compact_mode: boolean;
  };
  advanced: {
    log_level: 'Error' | 'Warn' | 'Info' | 'Debug' | 'Trace';
    max_log_size_mb: number;
    debug_mode: boolean;
    experimental_features: boolean;
    performance_mode: string;
    custom_ffmpeg_path?: string;
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