import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Play, Pause, Square, Mic, Video, Volume2 } from "lucide-react";

export default function Dashboard() {
  return (
    <div className="grid gap-6 md:grid-cols-2">
      {/* Current Status Card */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Video className="h-5 w-5" />
            Current Output
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="aspect-video bg-muted rounded-lg flex items-center justify-center">
            <p className="text-muted-foreground">No video selected</p>
          </div>

          <div className="flex items-center gap-4">
            <Button size="sm">
              <Play className="h-4 w-4 mr-2" />
              Play
            </Button>
            <Button size="sm" variant="outline">
              <Pause className="h-4 w-4 mr-2" />
              Pause
            </Button>
            <Button size="sm" variant="outline">
              <Square className="h-4 w-4 mr-2" />
              Stop
            </Button>
          </div>

          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <span className="text-sm font-medium">Volume</span>
              <Volume2 className="h-4 w-4" />
            </div>
            <div className="w-full bg-muted rounded-full h-2">
              <div className="bg-primary h-2 rounded-full w-3/4"></div>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Audio Status Card */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Mic className="h-5 w-5" />
            Audio Output
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="bg-muted rounded-lg p-8 text-center">
            <p className="text-muted-foreground">No audio selected</p>
          </div>

          <div className="flex items-center gap-4">
            <Button size="sm">
              <Play className="h-4 w-4 mr-2" />
              Play
            </Button>
            <Button size="sm" variant="outline">
              <Square className="h-4 w-4 mr-2" />
              Stop
            </Button>
            <Badge variant="secondary">Muted</Badge>
          </div>

          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <span className="text-sm font-medium">Audio Level</span>
              <span className="text-sm text-muted-foreground">0%</span>
            </div>
            <div className="w-full bg-muted rounded-full h-2">
              <div className="bg-primary h-2 rounded-full w-0"></div>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Recording Status */}
      <Card className="md:col-span-2">
        <CardHeader>
          <CardTitle>Recording Status</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-4">
              <Badge variant="outline">Not Recording</Badge>
              <span className="text-sm text-muted-foreground">Duration: 00:00:00</span>
            </div>
            <Button>
              <Square className="h-4 w-4 mr-2" />
              Start Recording
            </Button>
          </div>
        </CardContent>
      </Card>

      {/* Quick Actions */}
      <Card className="md:col-span-2">
        <CardHeader>
          <CardTitle>Quick Actions</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="grid gap-4 md:grid-cols-4">
            <Button variant="outline" className="h-20 flex-col">
              <Video className="h-6 w-6 mb-2" />
              Select Video
            </Button>
            <Button variant="outline" className="h-20 flex-col">
              <Mic className="h-6 w-6 mb-2" />
              Select Audio
            </Button>
            <Button variant="outline" className="h-20 flex-col">
              <Square className="h-6 w-6 mb-2" />
              Start Recording
            </Button>
            <Button variant="outline" className="h-20 flex-col">
              <Volume2 className="h-6 w-6 mb-2" />
              Mute All
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}