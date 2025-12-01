import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { FolderOpen, Monitor, Mic, Settings as SettingsIcon } from "lucide-react";
import HotkeyManager from "@/components/HotkeyManager";
import type { MediaDevice, AppSettings } from "@/types";

export default function Settings() {
  // Placeholder data - in real app this would come from Tauri commands
  const mockVideoDevices: MediaDevice[] = [];
  const mockAudioDevices: MediaDevice[] = [];
  const mockSettings: AppSettings = {
    mediaFolders: [],
    recording: {
      defaultResolution: "1080p",
      defaultQuality: "medium",
      outputPath: "",
    },
    ui: {
      theme: "system",
      autoStartRecording: false,
      showPreview: true,
    },
  };

  return (
    <div className="space-y-6">
      <Tabs defaultValue="devices" className="space-y-4">
        <TabsList className="grid w-full grid-cols-4">
          <TabsTrigger value="devices">Devices</TabsTrigger>
          <TabsTrigger value="media">Media Folders</TabsTrigger>
          <TabsTrigger value="hotkeys">Hotkeys</TabsTrigger>
          <TabsTrigger value="general">General</TabsTrigger>
        </TabsList>

        {/* Device Settings */}
        <TabsContent value="devices" className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Monitor className="h-5 w-5" />
                Virtual Webcam
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <div>
                <label className="text-sm font-medium">Select Virtual Webcam Device</label>
                <select className="w-full mt-1 p-2 border rounded-md">
                  <option value="">No device selected</option>
                  {mockVideoDevices.filter(d => d.isVirtual).map((device) => (
                    <option key={device.id} value={device.id}>
                      {device.name}
                    </option>
                  ))}
                </select>
              </div>
              <div className="text-sm text-muted-foreground">
                Install a virtual webcam driver like OBS Virtual Camera or ManyCam to enable this feature.
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Mic className="h-5 w-5" />
                Virtual Microphone
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <div>
                <label className="text-sm font-medium">Select Virtual Microphone Device</label>
                <select className="w-full mt-1 p-2 border rounded-md">
                  <option value="">No device selected</option>
                  {mockAudioDevices.filter(d => d.isVirtual).map((device) => (
                    <option key={device.id} value={device.id}>
                      {device.name}
                    </option>
                  ))}
                </select>
              </div>
              <div className="text-sm text-muted-foreground">
                Install a virtual audio driver like VB-CABLE or VoiceMeeter to enable this feature.
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        {/* Media Folders */}
        <TabsContent value="media" className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle>Media Folders</CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                {mockSettings.mediaFolders.length === 0 ? (
                  <div className="text-center py-8 text-muted-foreground">
                    No media folders configured
                  </div>
                ) : (
                  mockSettings.mediaFolders.map((folder, index) => (
                    <div key={index} className="flex items-center justify-between p-3 border rounded-md">
                      <div className="flex items-center gap-2">
                        <FolderOpen className="h-4 w-4" />
                        <span className="text-sm">{folder}</span>
                      </div>
                      <Button variant="outline" size="sm">
                        <Trash2 className="h-4 w-4" />
                      </Button>
                    </div>
                  ))
                )}
              </div>
              <Button className="w-full">
                <Plus className="h-4 w-4 mr-2" />
                Add Media Folder
              </Button>
            </CardContent>
          </Card>
        </TabsContent>

        {/* Hotkeys */}
        <TabsContent value="hotkeys" className="space-y-4">
          <HotkeyManager />
        </TabsContent>

        {/* General Settings */}
        <TabsContent value="general" className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle>Recording Settings</CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="grid gap-4 md:grid-cols-2">
                <div>
                  <label className="text-sm font-medium">Default Resolution</label>
                  <select className="w-full mt-1 p-2 border rounded-md">
                    <option value="720p">720p (1280x720)</option>
                    <option value="1080p">1080p (1920x1080)</option>
                  </select>
                </div>
                <div>
                  <label className="text-sm font-medium">Default Quality</label>
                  <select className="w-full mt-1 p-2 border rounded-md">
                    <option value="low">Low</option>
                    <option value="medium">Medium</option>
                    <option value="high">High</option>
                  </select>
                </div>
              </div>
              <div>
                <label className="text-sm font-medium">Recording Output Path</label>
                <div className="flex gap-2 mt-1">
                  <Input
                    placeholder="Select output folder..."
                    value={mockSettings.recording.outputPath}
                    readOnly
                  />
                  <Button variant="outline">
                    <FolderOpen className="h-4 w-4" />
                  </Button>
                </div>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>User Interface</CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-3">
                <label className="flex items-center justify-between">
                  <span className="text-sm font-medium">Show Preview</span>
                  <input type="checkbox" defaultChecked={mockSettings.ui.showPreview} />
                </label>
                <label className="flex items-center justify-between">
                  <span className="text-sm font-medium">Auto-start Recording</span>
                  <input type="checkbox" defaultChecked={mockSettings.ui.autoStartRecording} />
                </label>
              </div>
              <div>
                <label className="text-sm font-medium">Theme</label>
                <select className="w-full mt-1 p-2 border rounded-md">
                  <option value="light">Light</option>
                  <option value="dark">Dark</option>
                  <option value="system">System</option>
                </select>
              </div>
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  );
}