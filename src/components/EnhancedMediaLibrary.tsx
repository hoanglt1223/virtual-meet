import { useState, useEffect, useRef, useCallback } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Progress } from "@/components/ui/progress";
import { Alert, AlertDescription } from "@/components/ui/alert";
import {
  Search,
  Play,
  Plus,
  FolderOpen,
  RefreshCw,
  Trash2,
  Film,
  Music,
  Image as ImageIcon,
  FileText,
  AlertCircle,
  CheckCircle,
} from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

// Enhanced types matching our backend
interface MediaFile {
  path: string;
  name: string;
  file_type: "Video" | "Audio" | "Image" | "Unknown";
  size: number;
  duration?: number;
  metadata?: {
    video_info?: {
      width?: number;
      height?: number;
      fps?: number;
      codec?: string;
    };
    audio_info?: {
      channels?: number;
      sample_rate?: number;
      codec?: string;
    };
    format_info: {
      format_name: string;
      duration?: number;
      bit_rate?: number;
      file_size: number;
    };
  };
  thumbnail_path?: string;
  last_modified: string;
  created: string;
}

interface LibraryStatus {
  success: boolean;
  total_files: number;
  total_size_bytes: number;
  media_counts: {
    video_files: number;
    audio_files: number;
    image_files: number;
    other_files: number;
  };
  last_scan_time?: string;
  library_paths: string[];
}

interface ScanProgress {
  phase: "idle" | "scanning" | "processing" | "completed" | "error";
  message: string;
  progress?: number;
  files_found?: number;
  files_processed?: number;
  current_file?: string;
}

export default function EnhancedMediaLibrary() {
  const [mediaFiles, setMediaFiles] = useState<MediaFile[]>([]);
  const [filteredFiles, setFilteredFiles] = useState<MediaFile[]>([]);
  const [searchTerm, setSearchTerm] = useState("");
  const [selectedType, setSelectedType] = useState<"all" | "video" | "audio" | "image">("all");
  const [libraryStatus, setLibraryStatus] = useState<LibraryStatus | null>(null);
  const [scanProgress, setScanProgress] = useState<ScanProgress>({ phase: "idle", message: "" });
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selectedIndex, setSelectedIndex] = useState<number>(-1);
  const [focusedFile, setFocusedFile] = useState<string | null>(null);

  // Refs for keyboard navigation
  const searchInputRef = useRef<HTMLInputElement>(null);
  const mediaGridRef = useRef<HTMLDivElement>(null);
  const fileItemRefs = useRef<(HTMLDivElement | null)[]>([]);

  // Load library status on component mount
  useEffect(() => {
    loadLibraryStatus();
  }, []);

  // Filter files when search or type changes
  useEffect(() => {
    let filtered = mediaFiles;

    // Filter by type
    if (selectedType !== "all") {
      filtered = filtered.filter((file) =>
        file.file_type.toLowerCase() === selectedType.toLowerCase()
      );
    }

    // Filter by search term
    if (searchTerm) {
      filtered = filtered.filter((file) =>
        file.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
        file.path.toLowerCase().includes(searchTerm.toLowerCase())
      );
    }

    setFilteredFiles(filtered);
  }, [mediaFiles, searchTerm, selectedType]);

  // Reset selected index when filtered files change
  useEffect(() => {
    setSelectedIndex(-1);
    setFocusedFile(null);
    fileItemRefs.current = [];
  }, [filteredFiles]);

  // Keyboard navigation
  const handleKeyDown = useCallback((event: KeyboardEvent) => {
    // Ignore if user is typing in search input
    if (event.target === searchInputRef.current) {
      if (event.key === 'Escape') {
        searchInputRef.current?.blur();
        mediaGridRef.current?.focus();
      }
      return;
    }

    switch (event.key) {
      case 'ArrowDown':
        event.preventDefault();
        setSelectedIndex(prev => {
          const newIndex = Math.min(prev + 1, filteredFiles.length - 1);
          if (newIndex >= 0) {
            fileItemRefs.current[newIndex]?.focus();
            setFocusedFile(filteredFiles[newIndex]?.path || null);
          }
          return newIndex;
        });
        break;

      case 'ArrowUp':
        event.preventDefault();
        setSelectedIndex(prev => {
          const newIndex = Math.max(prev - 1, 0);
          if (newIndex >= 0) {
            fileItemRefs.current[newIndex]?.focus();
            setFocusedFile(filteredFiles[newIndex]?.path || null);
          }
          return newIndex;
        });
        break;

      case 'ArrowRight':
        event.preventDefault();
        if (filteredFiles.length > 0) {
          const cols = window.innerWidth >= 1024 ? 4 : window.innerWidth >= 768 ? 3 : 1;
          setSelectedIndex(prev => {
            const newIndex = Math.min(prev + cols, filteredFiles.length - 1);
            if (newIndex >= 0) {
              fileItemRefs.current[newIndex]?.focus();
              setFocusedFile(filteredFiles[newIndex]?.path || null);
            }
            return newIndex;
          });
        }
        break;

      case 'ArrowLeft':
        event.preventDefault();
        if (filteredFiles.length > 0) {
          const cols = window.innerWidth >= 1024 ? 4 : window.innerWidth >= 768 ? 3 : 1;
          setSelectedIndex(prev => {
            const newIndex = Math.max(prev - cols, 0);
            if (newIndex >= 0) {
              fileItemRefs.current[newIndex]?.focus();
              setFocusedFile(filteredFiles[newIndex]?.path || null);
            }
            return newIndex;
          });
        }
        break;

      case 'Enter':
        if (selectedIndex >= 0 && selectedIndex < filteredFiles.length) {
          const file = filteredFiles[selectedIndex];
          if (file.file_type === "Video" || file.file_type === "Audio") {
            setAsCurrentMedia(file, file.file_type.toLowerCase() as "video" | "audio");
          }
        }
        break;

      case '/':
        event.preventDefault();
        searchInputRef.current?.focus();
        break;

      case 'Escape':
        setSelectedIndex(-1);
        setFocusedFile(null);
        mediaGridRef.current?.focus();
        break;

      case 'a':
      case 'A':
        if (event.ctrlKey || event.metaKey) {
          event.preventDefault();
          setSelectedType("all");
        }
        break;

      case 'v':
      case 'V':
        if (event.ctrlKey || event.metaKey) {
          event.preventDefault();
          setSelectedType("video");
        }
        break;

      case 'l':
      case 'L':
        if (event.ctrlKey || event.metaKey) {
          event.preventDefault();
          setSelectedType("audio");
        }
        break;
    }
  }, [filteredFiles, selectedIndex]);

  // Setup keyboard event listeners
  useEffect(() => {
    document.addEventListener('keydown', handleKeyDown);
    return () => {
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, [handleKeyDown]);

  const loadLibraryStatus = async () => {
    try {
      const status: LibraryStatus = await invoke("get_media_library_status");
      setLibraryStatus(status);
    } catch (err) {
      console.error("Failed to load library status:", err);
      setError("Failed to load library status");
    }
  };

  const scanMediaLibrary = async () => {
    try {
      setError(null);
      setIsLoading(true);
      setScanProgress({ phase: "scanning", message: "Selecting folder to scan..." });

      // Open folder selection dialog
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Select Media Folder",
      });

      if (!selected) {
        setScanProgress({ phase: "idle", message: "" });
        setIsLoading(false);
        return;
      }

      setScanProgress({ phase: "scanning", message: "Initializing media library..." });

      // Initialize media library
      await invoke("initialize_media_library");

      setScanProgress({ phase: "scanning", message: "Scanning media files..." });

      // Start scan
      const scanRequest = {
        library_path: selected,
        force_refresh: true,
        include_subdirectories: true,
      };

      await invoke("load_media_library", { request: scanRequest });

      setScanProgress({ phase: "completed", message: "Scan completed successfully!" });

      // Reload library status
      await loadLibraryStatus();

      // Load the scanned files (in a real implementation, you'd get these from the scan result)
      await loadMediaFiles();

    } catch (err) {
      console.error("Scan failed:", err);
      setError(err instanceof Error ? err.message : "Scan failed");
      setScanProgress({ phase: "error", message: "Scan failed" });
    } finally {
      setIsLoading(false);
      // Clear progress after a delay
      setTimeout(() => {
        setScanProgress({ phase: "idle", message: "" });
      }, 3000);
    }
  };

  const loadMediaFiles = async () => {
    try {
      // In a real implementation, you'd have a command to get paginated media files
      // For now, we'll search with an empty query to get all files
      const searchRequest = {
        query: "",
        search_type: "All",
        max_results: 1000,
      };

      const result: any = await invoke("search_media_library_enhanced", { request: searchRequest });

      if (result.success) {
        setMediaFiles(result.results);
      }
    } catch (err) {
      console.error("Failed to load media files:", err);
    }
  };

  const setAsCurrentMedia = async (file: MediaFile, mediaType: "video" | "audio") => {
    try {
      if (mediaType === "video" && file.file_type === "Video") {
        await invoke("set_current_video", { videoPath: file.path });
      } else if (mediaType === "audio" && file.file_type === "Audio") {
        await invoke("set_current_audio", { audioPath: file.path });
      }
    } catch (err) {
      console.error(`Failed to set ${mediaType}:`, err);
      setError(`Failed to set ${mediaType} file`);
    }
  };

  const cleanupLibrary = async () => {
    try {
      setError(null);
      const result = await invoke("cleanup_media_library");
      console.log("Cleanup result:", result);
      // Reload status
      await loadLibraryStatus();
    } catch (err) {
      console.error("Cleanup failed:", err);
      setError("Failed to cleanup library");
    }
  };

  const formatFileSize = (bytes: number): string => {
    const units = ["B", "KB", "MB", "GB", "TB"];
    let size = bytes;
    let unitIndex = 0;

    while (size >= 1024 && unitIndex < units.length - 1) {
      size /= 1024;
      unitIndex++;
    }

    return `${size.toFixed(1)} ${units[unitIndex]}`;
  };

  const formatDuration = (seconds?: number): string => {
    if (!seconds) return "";
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}:${secs.toString().padStart(2, "0")}`;
  };

  const getMediaIcon = (fileType: string) => {
    switch (fileType) {
      case "Video":
        return <Film className="h-4 w-4" />;
      case "Audio":
        return <Music className="h-4 w-4" />;
      case "Image":
        return <ImageIcon className="h-4 w-4" />;
      default:
        return <FileText className="h-4 w-4" />;
    }
  };

  return (
    <div className="space-y-6">
      {/* Library Status Card */}
      {libraryStatus && (
        <Card>
          <CardHeader>
            <div className="flex items-center justify-between">
              <CardTitle className="flex items-center gap-2">
                <Film className="h-5 w-5" />
                Media Library
              </CardTitle>
              <div className="flex gap-2">
                <Button
                  variant="outline"
                  size="sm"
                  onClick={cleanupLibrary}
                  disabled={isLoading}
                >
                  <Trash2 className="h-4 w-4 mr-2" />
                  Cleanup
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={loadLibraryStatus}
                  disabled={isLoading}
                >
                  <RefreshCw className="h-4 w-4 mr-2" />
                  Refresh
                </Button>
              </div>
            </div>
          </CardHeader>
          <CardContent>
            <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-4">
              <div className="text-center">
                <div className="text-2xl font-bold">{libraryStatus.total_files}</div>
                <div className="text-sm text-muted-foreground">Total Files</div>
              </div>
              <div className="text-center">
                <div className="text-2xl font-bold">{libraryStatus.media_counts.video_files}</div>
                <div className="text-sm text-muted-foreground">Videos</div>
              </div>
              <div className="text-center">
                <div className="text-2xl font-bold">{libraryStatus.media_counts.audio_files}</div>
                <div className="text-sm text-muted-foreground">Audio</div>
              </div>
              <div className="text-center">
                <div className="text-2xl font-bold">{formatFileSize(libraryStatus.total_size_bytes)}</div>
                <div className="text-sm text-muted-foreground">Total Size</div>
              </div>
            </div>
            {libraryStatus.last_scan_time && (
              <div className="text-sm text-muted-foreground">
                Last scanned: {new Date(libraryStatus.last_scan_time).toLocaleString()}
              </div>
            )}
          </CardContent>
        </Card>
      )}

      {/* Scan Progress */}
      {scanProgress.phase !== "idle" && (
        <Alert>
          <div className="flex items-center gap-2">
            {scanProgress.phase === "completed" ? (
              <CheckCircle className="h-4 w-4 text-green-500" />
            ) : scanProgress.phase === "error" ? (
              <AlertCircle className="h-4 w-4 text-red-500" />
            ) : (
              <RefreshCw className="h-4 w-4 animate-spin" />
            )}
            <AlertDescription>{scanProgress.message}</AlertDescription>
          </div>
          {scanProgress.progress && (
            <Progress value={scanProgress.progress} className="mt-2" />
          )}
        </Alert>
      )}

      {/* Error Display */}
      {error && (
        <Alert variant="destructive">
          <AlertCircle className="h-4 w-4" />
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}

      {/* Keyboard Shortcuts Help */}
      <Card className="border-dashed">
        <CardHeader>
          <CardTitle className="text-sm">Keyboard Shortcuts</CardTitle>
        </CardHeader>
        <CardContent className="text-xs space-y-1 text-muted-foreground">
          <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-2">
            <div><kbd className="px-1 py-0.5 bg-muted rounded">↑↓←→</kbd> Navigate</div>
            <div><kbd className="px-1 py-0.5 bg-muted rounded">Enter</kbd> Select file</div>
            <div><kbd className="px-1 py-0.5 bg-muted rounded">/</kbd> Search</div>
            <div><kbd className="px-1 py-0.5 bg-muted rounded">Esc</kbd> Clear focus</div>
            <div><kbd className="px-1 py-0.5 bg-muted rounded">Ctrl+A</kbd> All files</div>
            <div><kbd className="px-1 py-0.5 bg-muted rounded">Ctrl+V</kbd> Videos</div>
            <div><kbd className="px-1 py-0.5 bg-muted rounded">Ctrl+L</kbd> Audio</div>
            <div><kbd className="px-1 py-0.5 bg-muted rounded">Space</kbd> Play/Pause</div>
          </div>
        </CardContent>
      </Card>

      {/* Search and Filter Bar */}
      <div className="flex flex-col gap-4 md:flex-row md:items-center md:justify-between">
        <div className="relative flex-1 max-w-md">
          <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 text-muted-foreground h-4 w-4" />
          <Input
            ref={searchInputRef}
            placeholder="Search media files... (Press / to focus)"
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            className="pl-10"
            aria-label="Search media files"
            autoComplete="off"
          />
        </div>

        <div className="flex gap-2">
          <Button
            variant={selectedType === "all" ? "default" : "outline"}
            size="sm"
            onClick={() => setSelectedType("all")}
          >
            All
          </Button>
          <Button
            variant={selectedType === "video" ? "default" : "outline"}
            size="sm"
            onClick={() => setSelectedType("video")}
          >
            <Film className="h-4 w-4 mr-1" />
            Videos
          </Button>
          <Button
            variant={selectedType === "audio" ? "default" : "outline"}
            size="sm"
            onClick={() => setSelectedType("audio")}
          >
            <Music className="h-4 w-4 mr-1" />
            Audio
          </Button>
          <Button
            variant={selectedType === "image" ? "default" : "outline"}
            size="sm"
            onClick={() => setSelectedType("image")}
          >
            <ImageIcon className="h-4 w-4 mr-1" />
            Images
          </Button>
        </div>

        <Button onClick={scanMediaLibrary} disabled={isLoading}>
          <FolderOpen className="h-4 w-4 mr-2" />
          {isLoading ? "Scanning..." : "Add Folder"}
        </Button>
      </div>

      {/* Media Grid */}
      <Card>
        <CardHeader>
          <CardTitle>
            Media Files ({filteredFiles.length})
            {searchTerm && ` matching "${searchTerm}"`}
          </CardTitle>
        </CardHeader>
        <CardContent>
          {filteredFiles.length === 0 ? (
            <div className="text-center py-12">
              <div className="text-muted-foreground mb-4">
                {searchTerm || selectedType !== "all"
                  ? "No media files match your filters"
                  : "No media files found"}
              </div>
              <Button onClick={scanMediaLibrary} disabled={isLoading}>
                <Plus className="h-4 w-4 mr-2" />
                Add Media Folder
              </Button>
            </div>
          ) : (
            <div
              ref={mediaGridRef}
              className="grid gap-4 md:grid-cols-3 lg:grid-cols-4"
              role="grid"
              aria-label="Media files"
              tabIndex={0}
            >
              {filteredFiles.map((file, index) => (
                <div
                  key={`${file.path}-${index}`}
                  ref={el => fileItemRefs.current[index] = el}
                  className={`border rounded-lg overflow-hidden hover:shadow-md transition-shadow cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500 ${
                    focusedFile === file.path ? 'ring-2 ring-blue-500' : ''
                  }`}
                  role="gridcell"
                  tabIndex={selectedIndex === index ? 0 : -1}
                  aria-selected={selectedIndex === index}
                  aria-label={`${file.file_type} file: ${file.name}`}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter' || e.key === ' ') {
                      e.preventDefault();
                      if (file.file_type === "Video" || file.file_type === "Audio") {
                        setAsCurrentMedia(file, file.file_type.toLowerCase() as "video" | "audio");
                      }
                    }
                  }}
                >
                  <div className="aspect-video bg-muted flex items-center justify-center relative">
                    {file.thumbnail_path ? (
                      <img
                        src={`file://${file.thumbnail_path}`}
                        alt={file.name}
                        className="w-full h-full object-cover"
                        onError={(e) => {
                          e.currentTarget.style.display = 'none';
                          e.currentTarget.nextElementSibling?.classList.remove('hidden');
                        }}
                      />
                    ) : null}
                    <div className={`${file.thumbnail_path ? 'hidden' : 'flex'} flex-col items-center justify-center text-4xl text-muted-foreground`}>
                      {getMediaIcon(file.file_type)}
                    </div>

                    {file.duration && (
                      <div className="absolute bottom-2 right-2 bg-black bg-opacity-75 text-white text-xs px-1 py-0.5 rounded">
                        {formatDuration(file.duration)}
                      </div>
                    )}
                  </div>

                  <div className="p-3">
                    <div className="font-medium text-sm truncate mb-2" title={file.name}>
                      {file.name}
                    </div>

                    <div className="flex items-center justify-between mb-2">
                      <Badge variant="secondary" className="text-xs flex items-center gap-1">
                        {getMediaIcon(file.file_type)}
                        {file.file_type}
                      </Badge>
                      <span className="text-xs text-muted-foreground">
                        {formatFileSize(file.size)}
                      </span>
                    </div>

                    {/* Metadata display */}
                    {file.metadata && (
                      <div className="text-xs text-muted-foreground mb-2 space-y-1">
                        {file.metadata.video_info && (
                          <div className="flex justify-between">
                            <span>Resolution:</span>
                            <span>{file.metadata.video_info.width}×{file.metadata.video_info.height}</span>
                          </div>
                        )}
                        {file.metadata.audio_info && (
                          <div className="flex justify-between">
                            <span>Audio:</span>
                            <span>{file.metadata.audio_info.channels}ch, {file.metadata.audio_info.sample_rate}Hz</span>
                          </div>
                        )}
                      </div>
                    )}

                    <div className="flex gap-1">
                      {(file.file_type === "Video" || file.file_type === "Audio") && (
                        <Button
                          size="sm"
                          variant="outline"
                          className="flex-1"
                          onClick={() => setAsCurrentMedia(file, file.file_type.toLowerCase() as "video" | "audio")}
                        >
                          <Play className="h-3 w-3 mr-1" />
                          Use
                        </Button>
                      )}
                    </div>
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