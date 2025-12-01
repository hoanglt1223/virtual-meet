import { useState } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Progress } from "@/components/ui/progress";
import { Square, Circle, Download, Trash2 } from "lucide-react";
import type { Recording } from "@/types";

export default function Recording() {
  const [isRecording, setIsRecording] = useState(false);
  const [recordingTime] = useState(0);

  // Placeholder data - in real app this would come from Tauri commands
  const mockRecordings: Recording[] = [];

  const formatTime = (seconds: number) => {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    const secs = seconds % 60;
    return `${hours.toString().padStart(2, '0')}:${minutes.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
  };

  return (
    <div className="space-y-6">
      {/* Recording Controls */}
      <Card>
        <CardHeader>
          <CardTitle>Recording Controls</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center gap-4">
            <Button
              onClick={() => setIsRecording(!isRecording)}
              className={isRecording ? "bg-destructive hover:bg-destructive/90" : ""}
              disabled={!isRecording && recordingTime > 0}
            >
              {isRecording ? (
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

            {isRecording && (
              <Badge variant="destructive" className="animate-pulse">
                Recording
              </Badge>
            )}
          </div>

          <div className="space-y-2">
            <div className="flex justify-between text-sm">
              <span>Duration</span>
              <span>{formatTime(recordingTime)}</span>
            </div>
            <Progress value={(recordingTime % 60) * 1.67} className="h-2" />
          </div>

          <div className="grid gap-4 md:grid-cols-3">
            <div>
              <label className="text-sm font-medium">Resolution</label>
              <select className="w-full mt-1 p-2 border rounded-md">
                <option value="720p">720p (1280x720)</option>
                <option value="1080p">1080p (1920x1080)</option>
              </select>
            </div>
            <div>
              <label className="text-sm font-medium">Quality</label>
              <select className="w-full mt-1 p-2 border rounded-md">
                <option value="low">Low</option>
                <option value="medium">Medium</option>
                <option value="high">High</option>
              </select>
            </div>
            <div>
              <label className="text-sm font-medium">Output Folder</label>
              <div className="flex gap-2 mt-1">
                <input
                  type="text"
                  className="flex-1 p-2 border rounded-md"
                  placeholder="Select output folder..."
                  readOnly
                />
                <Button variant="outline" size="sm">
                  Browse
                </Button>
              </div>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Recent Recordings */}
      <Card>
        <CardHeader>
          <CardTitle>Recent Recordings ({mockRecordings.length})</CardTitle>
        </CardHeader>
        <CardContent>
          {mockRecordings.length === 0 ? (
            <div className="text-center py-12">
              <div className="text-muted-foreground mb-4">
                No recordings yet. Start recording to see them here.
              </div>
            </div>
          ) : (
            <div className="space-y-4">
              {mockRecordings.map((recording) => (
                <div
                  key={recording.id}
                  className="flex items-center justify-between p-4 border rounded-lg"
                >
                  <div className="flex-1">
                    <div className="font-medium">{recording.filename}</div>
                    <div className="text-sm text-muted-foreground">
                      {formatTime(recording.duration)} • {recording.resolution} • {recording.quality} • {(recording.fileSize / 1024 / 1024).toFixed(1)} MB
                    </div>
                    <div className="text-xs text-muted-foreground">
                      {new Date(recording.createdAt).toLocaleString()}
                    </div>
                  </div>
                  <div className="flex gap-2">
                    <Button variant="outline" size="sm">
                      <Download className="h-4 w-4 mr-2" />
                      Open
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
    </div>
  );
}