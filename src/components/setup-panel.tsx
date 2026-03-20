import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  CheckCircle, XCircle, AlertTriangle, ExternalLink,
  Camera, Mic, RefreshCw, Loader2, Monitor
} from "lucide-react";

interface SetupRecommendation {
  category: string;
  status: string;
  message: string;
  action_url?: string;
}

interface VirtualDeviceSetup {
  virtual_cameras: string[];
  virtual_audio_cables: string[];
  ffmpeg_available: boolean;
  obs_camera_detected: boolean;
  vb_cable_detected: boolean;
  voicemeeter_detected: boolean;
  windows_build: number;
  imf_virtual_camera_supported: boolean;
  recommendations: SetupRecommendation[];
}

export default function SetupPanel() {
  const [setup, setSetup] = useState<VirtualDeviceSetup | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const detectDevices = async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<VirtualDeviceSetup>("detect_virtual_devices");
      setSetup(result);
    } catch (e) {
      setError(`Detection failed: ${e}`);
    }
    setLoading(false);
  };

  useEffect(() => {
    const init = async () => {
      setLoading(true);
      try {
        const result = await invoke<VirtualDeviceSetup>("detect_virtual_devices");
        setSetup(result);
      } catch (e) {
        setError(`Detection failed: ${e}`);
      }
      setLoading(false);
    };
    init();
  }, []);

  const statusIcon = (status: string) => {
    switch (status) {
      case "ready": return <CheckCircle className="h-4 w-4 text-green-500" />;
      case "missing": return <XCircle className="h-4 w-4 text-red-500" />;
      case "optional": return <AlertTriangle className="h-4 w-4 text-yellow-500" />;
      default: return null;
    }
  };

  if (loading) {
    return (
      <Card>
        <CardContent className="py-8 flex items-center justify-center gap-2">
          <Loader2 className="h-5 w-5 animate-spin" />
          <span>Detecting virtual devices...</span>
        </CardContent>
      </Card>
    );
  }

  if (error) {
    return (
      <Card>
        <CardContent className="py-4 text-destructive">{error}</CardContent>
      </Card>
    );
  }

  if (!setup) return null;

  const allReady = setup.recommendations.every(r => r.status === "ready");

  return (
    <div className="space-y-4">
      {/* Overall Status */}
      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <Monitor className="h-5 w-5" />
              Device Setup
            </div>
            <Button size="sm" variant="outline" onClick={detectDevices}>
              <RefreshCw className="h-3 w-3 mr-1" />
              Refresh
            </Button>
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          {allReady ? (
            <div className="flex items-center gap-2 text-sm text-green-600">
              <CheckCircle className="h-4 w-4" />
              All virtual devices ready! You can start streaming.
            </div>
          ) : (
            <div className="flex items-center gap-2 text-sm text-yellow-600">
              <AlertTriangle className="h-4 w-4" />
              Some virtual devices are missing. See recommendations below.
            </div>
          )}

          {/* Device Status Grid */}
          <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
            <div className="bg-muted rounded-lg p-3 text-sm">
              <div className="flex items-center gap-1 text-muted-foreground mb-1">
                <Camera className="h-3 w-3" /> Virtual Cameras
              </div>
              <div className="font-medium">
                {setup.virtual_cameras.length > 0 ? (
                  <Badge variant="default" className="text-xs">{setup.virtual_cameras.length} found</Badge>
                ) : (
                  <Badge variant="destructive" className="text-xs">None</Badge>
                )}
              </div>
            </div>
            <div className="bg-muted rounded-lg p-3 text-sm">
              <div className="flex items-center gap-1 text-muted-foreground mb-1">
                <Mic className="h-3 w-3" /> Audio Cables
              </div>
              <div className="font-medium">
                {setup.virtual_audio_cables.length > 0 ? (
                  <Badge variant="default" className="text-xs">{setup.virtual_audio_cables.length} found</Badge>
                ) : (
                  <Badge variant="destructive" className="text-xs">None</Badge>
                )}
              </div>
            </div>
            <div className="bg-muted rounded-lg p-3 text-sm">
              <div className="flex items-center gap-1 text-muted-foreground mb-1">
                FFmpeg
              </div>
              <div className="font-medium">
                {setup.ffmpeg_available ? (
                  <Badge variant="default" className="text-xs">Available</Badge>
                ) : (
                  <Badge variant="destructive" className="text-xs">Missing</Badge>
                )}
              </div>
            </div>
            <div className="bg-muted rounded-lg p-3 text-sm">
              <div className="flex items-center gap-1 text-muted-foreground mb-1">
                Windows Build
              </div>
              <div className="font-medium">
                <Badge variant={setup.imf_virtual_camera_supported ? "default" : "secondary"} className="text-xs">
                  {setup.windows_build}
                </Badge>
              </div>
            </div>
          </div>

          {/* Recommendations */}
          {setup.recommendations.length > 0 && (
            <div className="space-y-2">
              {setup.recommendations.map((rec, i) => (
                <div key={i} className="flex items-start gap-2 text-sm p-2 rounded bg-muted/50">
                  {statusIcon(rec.status)}
                  <div className="flex-1">
                    <span>{rec.message}</span>
                  </div>
                  {rec.action_url && (
                    <Button
                      size="sm"
                      variant="outline"
                      className="h-7 text-xs shrink-0"
                      onClick={() => window.open(rec.action_url!, "_blank")}
                    >
                      <ExternalLink className="h-3 w-3 mr-1" />
                      Install
                    </Button>
                  )}
                </div>
              ))}
            </div>
          )}

          {/* Detected Devices List */}
          {(setup.virtual_cameras.length > 0 || setup.virtual_audio_cables.length > 0) && (
            <details className="text-sm">
              <summary className="cursor-pointer text-muted-foreground hover:text-foreground">
                Show detected devices
              </summary>
              <div className="mt-2 space-y-1 pl-4">
                {setup.virtual_cameras.map((d, i) => (
                  <div key={`vc-${i}`} className="flex items-center gap-2">
                    <Camera className="h-3 w-3 text-muted-foreground" />
                    <span>{d}</span>
                  </div>
                ))}
                {setup.virtual_audio_cables.map((d, i) => (
                  <div key={`ac-${i}`} className="flex items-center gap-2">
                    <Mic className="h-3 w-3 text-muted-foreground" />
                    <span>{d}</span>
                  </div>
                ))}
              </div>
            </details>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
